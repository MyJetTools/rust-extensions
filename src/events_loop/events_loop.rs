use std::{sync::Arc, time::Duration};

use tokio::sync::Mutex;

use crate::ApplicationStates;

use super::{EventsLoopLogger, EventsLoopTick};

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
    tick: Mutex<Option<Arc<dyn EventsLoopTick<TModel> + Send + Sync + 'static>>>,
    iteration_timeout: Duration,
    receiver: Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<EventsLoopMessage<TModel>>>>,
    sender: tokio::sync::mpsc::UnboundedSender<EventsLoopMessage<TModel>>,
    name: String,
}

impl<TModel: Send + Sync + 'static> EventsLoop<TModel> {
    pub fn new(name: String) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        Self {
            iteration_timeout: Duration::from_secs(5),
            receiver: Mutex::new(Some(receiver)),
            sender,
            tick: Mutex::new(None),
            name,
        }
    }

    pub async fn register_event_loop(
        &self,
        event_loop: Arc<dyn EventsLoopTick<TModel> + Send + Sync + 'static>,
    ) {
        let mut write_access = self.tick.lock().await;
        *write_access = Some(event_loop);
    }

    pub fn send(&self, model: TModel) {
        if let Err(_) = self.sender.send(EventsLoopMessage::NewMessage(model)) {
            println!("Can not send model to event loop {}", self.name);
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

    pub async fn start<TLogger: EventsLoopLogger + Send + Sync + 'static>(
        &self,
        app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
        logger: Arc<TLogger>,
    ) {
        let receiver = self.get_receiver().await;

        let event_loop = {
            let read_access = self.tick.lock().await;
            if read_access.is_none() {
                panic!("Event Loop is not registered");
            }

            read_access.as_ref().unwrap().clone()
        };

        tokio::spawn(events_loop_reader(
            self.name.clone(),
            event_loop,
            app_states,
            logger.clone(),
            self.iteration_timeout,
            receiver,
        ));
    }

    pub fn stop(&self) {
        if let Err(_) = self.sender.send(EventsLoopMessage::Shutdown) {
            println!("Can not send shutdown message to event loop {}", self.name);
        }
    }
}

async fn events_loop_reader<
    TModel: Send + Sync + 'static,
    TLogger: EventsLoopLogger + Send + Sync + 'static,
>(
    name: String,
    event_loop_tick: Arc<dyn EventsLoopTick<TModel> + Send + Sync + 'static>,
    app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
    logger: Arc<TLogger>,
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
                    if let Err(err) = result {
                        let message =
                            format!("EventLoop {} is panicked. Err: {:?}", name.as_str(), err);

                        let logger = logger.clone();

                        let name = name.clone();

                        tokio::spawn(async move {
                            println!("{}", message);
                            logger.write_error(name, message);
                        });
                    }
                }
                Err(err) => {
                    println!("Timer {} is timeouted with err: {:?}", name.as_str(), err);
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
