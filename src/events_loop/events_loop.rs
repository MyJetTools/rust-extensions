use std::{sync::Arc, time::Duration};

use tokio::sync::Mutex;

use crate::{ApplicationStates, Logger};

use super::EventsLoopTick;

pub enum EventsLoopMessage<TModel: Send + Sync + 'static> {
    NewMessage(TModel),
    Shutdown,
}

impl<TModel: Send + Sync + 'static> EventsLoopMessage<TModel> {
    pub fn is_shutdown(&self) -> bool {
        match self {
            EventsLoopMessage::Shutdown => true,
            _ => false,
        }
    }

    pub fn unwrap_message(self) -> TModel {
        match self {
            EventsLoopMessage::NewMessage(message) => message,
            _ => panic!("EventsLoopMessage::unwrap_message() called on a non-NewMessage message"),
        }
    }
}

pub struct EventsLoop<TModel: Send + Sync + 'static> {
    event_loop: Mutex<Option<Arc<dyn EventsLoopTick<TModel> + Send + Sync + 'static>>>,
    iteration_timeout: Duration,
    receiver: Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<EventsLoopMessage<TModel>>>>,
    sender: tokio::sync::mpsc::UnboundedSender<EventsLoopMessage<TModel>>,
    name: Arc<String>,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
}

impl<TModel: Send + Sync + 'static> EventsLoop<TModel> {
    pub fn new(name: String, logger: Arc<dyn Logger + Send + Sync + 'static>) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        Self {
            iteration_timeout: Duration::from_secs(5),
            receiver: Mutex::new(Some(receiver)),
            sender,
            event_loop: Mutex::new(None),
            name: Arc::new(name),
            logger,
        }
    }

    pub fn set_iteration_timeout(mut self, timeout: Duration) -> Self {
        self.iteration_timeout = timeout;
        self
    }

    pub async fn register_event_loop(
        &self,
        event_loop: Arc<dyn EventsLoopTick<TModel> + Send + Sync + 'static>,
    ) {
        let mut write_access = self.event_loop.lock().await;
        *write_access = Some(event_loop);
    }

    pub fn send(&self, model: TModel) {
        if let Err(_) = self.sender.send(EventsLoopMessage::NewMessage(model)) {
            println!("Can not send model to event loop {}", self.name.as_str());
        }
    }

    async fn get_receiver(
        &self,
    ) -> tokio::sync::mpsc::UnboundedReceiver<EventsLoopMessage<TModel>> {
        let mut write_access = self.receiver.lock().await;

        let mut result = None;
        std::mem::swap(&mut *write_access, &mut result);

        if result.is_none() {
            panic!("You can not start EventsLoop twice");
        }

        result.unwrap()
    }

    pub async fn start(&self, app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>) {
        let receiver = self.get_receiver().await;

        let event_loop = {
            let mut write_access = self.event_loop.lock().await;

            let event_loop = write_access.take();
            if write_access.is_none() {
                panic!("Event Loop is not registered");
            }

            event_loop.unwrap()
        };

        let logger = self.logger.clone();
        tokio::spawn(events_loop_reader(
            self.name.clone(),
            event_loop,
            app_states,
            logger,
            self.iteration_timeout,
            receiver,
        ));
    }

    pub fn stop(&self) {
        if let Err(err) = self.sender.send(EventsLoopMessage::Shutdown) {
            self.logger.write_error(
                format!("Stop EventLoop {}", self.name),
                format!("Can not send shutdown message to event loop {:?}", err),
                None.into(),
            );
        }
    }
}

async fn events_loop_reader<TModel: Send + Sync + 'static>(
    name: Arc<String>,
    event_loop_tick: Arc<dyn EventsLoopTick<TModel> + Send + Sync + 'static>,
    app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
    iteration_timeout: Duration,
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<EventsLoopMessage<TModel>>,
) {
    while !app_states.is_initialized() {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    while !app_states.is_shutting_down() {
        if let Some(message) = tokio::sync::mpsc::UnboundedReceiver::recv(&mut receiver).await {
            if message.is_shutdown() {
                return;
            }

            let timer_tick = tokio::spawn(execute_timer(
                event_loop_tick.clone(),
                message.unwrap_message(),
            ));
            match tokio::time::timeout(iteration_timeout, timer_tick).await {
                Ok(result) => {
                    if let Err(_) = result {
                        logger.write_error(
                            format!("EventLoop {} iteration", name.as_str()),
                            format!("Iteration is panicked"),
                            None.into(),
                        );
                    }
                }
                Err(_) => {
                    logger.write_error(
                        format!("EventLoop {} iteration", name.as_str()),
                        format!("Iteration is time outed"),
                        None.into(),
                    );
                }
            }
        }
    }
}

async fn execute_timer<TModel: Send + Sync + 'static>(
    events_loop_tick: Arc<dyn EventsLoopTick<TModel> + Send + Sync + 'static>,
    model: TModel,
) {
    events_loop_tick.tick(model).await;
}
