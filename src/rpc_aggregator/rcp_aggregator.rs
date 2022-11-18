use std::sync::{atomic::AtomicUsize, Arc};

use tokio::sync::Mutex;

use crate::{ApplicationStates, Logger, RpcAggregatorCallback, TaskCompletion};

use super::{
    rcp_aggregator_inner::{Request, RpcAggregatorInner},
    rpc_request_data::RcpRequestData,
};

pub struct RpcAggregator<
    TItem: Send + Sync + 'static,
    TResult: Send + Sync + 'static,
    TError: Send + Sync + 'static,
> {
    inner: Arc<(
        Mutex<RpcAggregatorInner<TItem, TResult, TError>>,
        AtomicUsize,
    )>,
    sender: tokio::sync::mpsc::UnboundedSender<()>,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
    name: String,
    max_amount_per_round_trip: usize,
    app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
    pub tick_timeout: std::time::Duration,
}

impl<
        TItem: Send + Sync + 'static,
        TResult: Send + Sync + 'static,
        TError: Send + Sync + 'static,
    > RpcAggregator<TItem, TResult, TError>
{
    pub fn new(
        name: String,
        max_amount_per_round_trip: usize,
        app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
        logger: Arc<dyn Logger + Send + Sync + 'static>,
    ) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        Self {
            inner: Arc::new((
                Mutex::new(RpcAggregatorInner::new(receiver)),
                AtomicUsize::new(0),
            )),
            sender,
            logger,
            name,
            max_amount_per_round_trip,
            tick_timeout: std::time::Duration::from_secs(10),
            app_states,
        }
    }

    pub fn get_count(&self) -> usize {
        self.inner.1.load(std::sync::atomic::Ordering::Relaxed)
    }

    async fn get_receiver(&self) -> tokio::sync::mpsc::UnboundedReceiver<()> {
        let mut write_access = self.inner.0.lock().await;
        let result = write_access.receiver.take();

        if result.is_none() {
            panic!("You can not start RoundTripPusher twice");
        }

        result.unwrap()
    }

    pub async fn start(
        &self,
        callback: Arc<dyn RpcAggregatorCallback<TItem, TResult, TError> + Send + Sync + 'static>,
    ) {
        let receiver = self.get_receiver().await;

        let name = self.name.clone();
        tokio::spawn(read_loop(
            name,
            self.inner.clone(),
            self.logger.clone(),
            callback,
            self.max_amount_per_round_trip,
            self.tick_timeout,
            receiver,
        ));
    }

    pub async fn execute_request(&self, data: TItem) -> Result<TResult, Arc<TError>> {
        if self.app_states.is_shutting_down() {
            panic!(
                "Can not publish to RoundTripPusher {} when shutting down",
                self.name
            );
        }

        let mut event = Request {
            request_data: data,
            completion: TaskCompletion::new(),
        };

        let task_await = event.completion.get_awaiter();

        {
            let mut write_access = self.inner.0.lock().await;

            write_access.queue.push(event);
            self.inner.1.store(
                write_access.queue.len(),
                std::sync::atomic::Ordering::SeqCst,
            );
        }
        if self.sender.send(()).is_err() {
            self.logger.write_fatal_error(
                format!("publish to pusher {}", self.name),
                "can not send".to_string(),
                None,
            );
        }

        task_await.get_result().await
    }

    pub async fn execute_multi_requests(
        &self,
        data: Vec<TItem>,
    ) -> Vec<Result<TResult, Arc<TError>>> {
        if self.app_states.is_shutting_down() {
            panic!(
                "Can not publish to RoundTripPusher {} when shutting down",
                self.name
            );
        }

        let mut awaiters = Vec::with_capacity(data.len());

        {
            let mut write_access = self.inner.0.lock().await;

            for data in data {
                let mut event = Request {
                    request_data: data,
                    completion: TaskCompletion::new(),
                };

                let task_await = event.completion.get_awaiter();
                write_access.queue.push(event);

                awaiters.push(task_await)
            }

            self.inner.1.store(
                write_access.queue.len(),
                std::sync::atomic::Ordering::SeqCst,
            );
        }
        if self.sender.send(()).is_err() {
            self.logger.write_fatal_error(
                format!("publish to pusher {}", self.name),
                "can not send".to_string(),
                None,
            );
        }

        let mut result = Vec::with_capacity(awaiters.len());

        for awaiter in awaiters {
            result.push(awaiter.get_result().await);
        }

        result
    }
}

async fn read_loop<
    TItem: Send + Sync + 'static,
    TResult: Send + Sync + 'static,
    TError: Send + Sync + 'static,
>(
    name: String,
    inner: Arc<(
        Mutex<RpcAggregatorInner<TItem, TResult, TError>>,
        AtomicUsize,
    )>,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
    callback: Arc<dyn RpcAggregatorCallback<TItem, TResult, TError> + Send + Sync + 'static>,
    max_amount_per_round_trip: usize,
    tick_timeout: std::time::Duration,
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<()>,
) {
    loop {
        let to_publish = {
            let mut write_access = inner.0.lock().await;

            if write_access.queue.len() == 0 {
                inner.1.store(
                    write_access.queue.len(),
                    std::sync::atomic::Ordering::SeqCst,
                );
                None
            } else if write_access.queue.len() > max_amount_per_round_trip {
                let mut to_yield = Vec::with_capacity(max_amount_per_round_trip);

                while to_yield.len() < max_amount_per_round_trip {
                    to_yield.push(write_access.queue.remove(0));
                }

                inner.1.store(
                    write_access.queue.len(),
                    std::sync::atomic::Ordering::SeqCst,
                );

                Some(RcpRequestData::new(to_yield))
            } else {
                let mut result = Vec::new();
                std::mem::swap(&mut write_access.queue, &mut result);

                inner.1.store(
                    write_access.queue.len(),
                    std::sync::atomic::Ordering::SeqCst,
                );
                Some(RcpRequestData::new(result))
            }
        };

        if let Some(mut to_publish) = to_publish {
            let data_to_callback = to_publish.get_data_to_callback();

            let mut attempt_no = 0;
            loop {
                let cloned = data_to_callback.clone();
                let callback = callback.clone();

                let future = tokio::spawn(async move { callback.handle(cloned.as_ref()).await });

                let result = tokio::time::timeout(tick_timeout, future).await;

                attempt_no += 1;

                if result.is_err() {
                    if attempt_no >= 5 {
                        logger.write_fatal_error(
                            format!("round trip pusher {}", name),
                            format!("Attempt {}. Skipping items", attempt_no),
                            None,
                        );

                        to_publish.set_panic("Timeout");

                        break;
                    }

                    logger.write_fatal_error(
                        format!("round trip pusher {}", name),
                        format!("Attempt {} timeout", attempt_no),
                        None,
                    );

                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }

                let result = result.unwrap();

                if let Err(err) = &result {
                    if attempt_no >= 5 {
                        logger.write_fatal_error(
                            format!("round trip pusher {}", name),
                            format!("Attempt {}. Skipping items", attempt_no),
                            None,
                        );

                        to_publish.set_panic(format!("{}", err).as_str());

                        break;
                    }

                    logger.write_fatal_error(
                        format!("round trip pusher {}", name),
                        format!("Attempt {} panic. Err: {:?}", attempt_no, err),
                        None,
                    );

                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }

                match result.unwrap() {
                    Ok(results) => {
                        if let Err(message) = to_publish.set_results(results) {
                            to_publish.set_panic(message.as_str());
                        }
                        break;
                    }
                    Err(err) => {
                        to_publish.set_error(err);
                        break;
                    }
                }
            }
        } else {
            receiver.recv().await;
        }
    }
}
