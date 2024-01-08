use std::{sync::Arc, time::Duration};

use crate::{ApplicationStates, Logger};

use super::{EventsLoopInner, EventsLoopTick};

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
    inner: EventsLoopInner<TModel>,
    iteration_timeout: Duration,
    name: Arc<String>,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
}

impl<TModel: Send + Sync + 'static> EventsLoop<TModel> {
    pub fn new(name: String, logger: Arc<dyn Logger + Send + Sync + 'static>) -> Self {
        Self {
            iteration_timeout: Duration::from_secs(5),
            inner: EventsLoopInner::new(),
            name: Arc::new(name),
            logger,
        }
    }

    pub fn set_iteration_timeout(mut self, timeout: Duration) -> Self {
        self.iteration_timeout = timeout;
        self
    }

    pub fn register_event_loop(
        &mut self,
        event_loop: Arc<dyn EventsLoopTick<TModel> + Send + Sync + 'static>,
    ) {
        if self.inner.events_loop_tick.is_some() {
            panic!("Event Loop is already registered");
        }
        self.inner.events_loop_tick = Some(event_loop);
    }

    pub fn send(&self, model: TModel) {
        if let Some(sender) = self.inner.sender.as_ref() {
            if let Err(_) = sender.send(EventsLoopMessage::NewMessage(model)) {
                println!("Can not send model to event loop {}", self.name.as_str());
            }
        }
    }

    fn get_receiver(&mut self) -> tokio::sync::mpsc::UnboundedReceiver<EventsLoopMessage<TModel>> {
        if self.inner.sender.is_some() {
            panic!("You can not start EventsLoop twice");
        }

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        self.inner.sender = Some(sender);
        receiver
    }

    pub fn start(&mut self, app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>) {
        let receiver = self.get_receiver();

        let event_loop = self.inner.events_loop_tick.take();
        if event_loop.is_none() {
            panic!("Event Loop is not registered");
        }

        let event_loop = event_loop.unwrap();

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
        if let Some(sender) = self.inner.sender.as_ref() {
            if let Err(err) = sender.send(EventsLoopMessage::Shutdown) {
                self.logger.write_error(
                    format!("Stop EventLoop {}", self.name),
                    format!("Can not send shutdown message to event loop {:?}", err),
                    None.into(),
                );
            }
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

    let event_loop_tick_spawned = event_loop_tick.clone();
    let _ = tokio::spawn(async move {
        event_loop_tick_spawned.started().await;
    })
    .await;

    while !app_states.is_shutting_down() {
        if let Some(message) = tokio::sync::mpsc::UnboundedReceiver::recv(&mut receiver).await {
            if message.is_shutdown() {
                break;
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

    let event_loop_tick_spawned = event_loop_tick.clone();
    let _ = tokio::spawn(async move {
        event_loop_tick_spawned.finished().await;
    })
    .await;
}

async fn execute_timer<TModel: Send + Sync + 'static>(
    events_loop_tick: Arc<dyn EventsLoopTick<TModel> + Send + Sync + 'static>,
    model: TModel,
) {
    events_loop_tick.tick(model).await;
}
