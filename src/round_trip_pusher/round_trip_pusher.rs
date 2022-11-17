use std::sync::{atomic::AtomicUsize, Arc};

use tokio::sync::Mutex;

use crate::{ApplicationStates, Logger, RoundTripCallback};

use super::round_trip_pusher_inner::RoundTripPusherInner;

pub struct RoundTripPusher<TItem: Send + Sync + 'static> {
    inner: Arc<(Mutex<RoundTripPusherInner<TItem>>, AtomicUsize)>,
    sender: tokio::sync::mpsc::UnboundedSender<()>,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
    name: String,
    max_amount_per_round_trip: usize,
    app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
    pub tick_timeout: std::time::Duration,
}

impl<TItem: Send + Sync + 'static> RoundTripPusher<TItem> {
    pub fn new(
        name: String,
        max_amount_per_round_trip: usize,
        app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
        logger: Arc<dyn Logger + Send + Sync + 'static>,
    ) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        Self {
            inner: Arc::new((
                Mutex::new(RoundTripPusherInner::new(receiver)),
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

    pub async fn start(&self, callback: Arc<dyn RoundTripCallback<TItem> + Send + Sync + 'static>) {
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

    pub async fn publish(&self, item: TItem) {
        if self.app_states.is_shutting_down() {
            panic!(
                "Can not publish to RoundTripPusher {} when shutting down",
                self.name
            );
        }

        {
            let mut write_access = self.inner.0.lock().await;
            write_access.queue.push(item);
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
    }

    pub async fn publish_many<TIter: Iterator<Item = TItem>>(&self, items: TIter) {
        if self.app_states.is_shutting_down() {
            panic!(
                "Can not publish to RoundTripPusher {} when shutting down",
                self.name
            );
        }

        {
            let mut write_access = self.inner.0.lock().await;
            write_access.queue.extend(items);
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
    }
}

async fn read_loop<TItem: Send + Sync + 'static>(
    name: String,
    inner: Arc<(Mutex<RoundTripPusherInner<TItem>>, AtomicUsize)>,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
    callback: Arc<dyn RoundTripCallback<TItem> + Send + Sync + 'static>,
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

                Some(to_yield)
            } else {
                let mut result = Vec::new();
                std::mem::swap(&mut write_access.queue, &mut result);

                inner.1.store(
                    write_access.queue.len(),
                    std::sync::atomic::Ordering::SeqCst,
                );
                Some(result)
            }
        };

        if let Some(to_publish) = to_publish {
            let to_publish = Arc::new(to_publish);
            let mut attempt_no = 0;
            loop {
                let cloned = to_publish.clone();
                let callback = callback.clone();

                let future = tokio::spawn(async move {
                    callback.handle(cloned.as_ref()).await;
                });

                let result = tokio::time::timeout(tick_timeout, future).await;

                attempt_no += 1;

                if attempt_no >= 5 {
                    logger.write_fatal_error(
                        format!("round trip pusher {}", name),
                        format!("Attempt {}. Skipping items", attempt_no),
                        None,
                    );
                    break;
                }

                if result.is_err() {
                    logger.write_fatal_error(
                        format!("round trip pusher {}", name),
                        format!("Attempt {} timeout", attempt_no),
                        None,
                    );

                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }

                let result = result.unwrap();

                if let Err(err) = result {
                    logger.write_fatal_error(
                        format!("round trip pusher {}", name),
                        format!("Attempt {} panic. Err: {:?}", attempt_no, err),
                        None,
                    );

                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }

                break;
            }
        } else {
            receiver.recv().await;
        }
    }
}
