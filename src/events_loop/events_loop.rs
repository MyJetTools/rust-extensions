use std::{sync::Arc, time::Duration};

use crate::{ApplicationStates, Logger};

use super::{EventsLoopInner, EventsLoopMode, EventsLoopPublisher, EventsLoopTick};

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
        let name = Arc::new(name);
        Self {
            iteration_timeout: Duration::from_secs(5),
            inner: EventsLoopInner::new(),
            name,
            logger,
        }
    }

    pub fn get_publisher(&mut self) -> EventsLoopPublisher<TModel> {
        match &self.inner.mode {
            EventsLoopMode::Unknown => {}
            EventsLoopMode::NoExternalPublisher(_) => {
                panic!("Event loop is already running")
            }
            EventsLoopMode::Publisher(_) => {
                panic!("Publisher already created")
            }
        }

        let (publisher, receiver) =
            EventsLoopPublisher::new(self.name.clone(), self.logger.clone());
        self.inner.mode = EventsLoopMode::Publisher(Some(receiver));
        publisher
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
        match &self.inner.mode {
            EventsLoopMode::Unknown => {
                panic!("Event loop is not running")
            }
            EventsLoopMode::NoExternalPublisher(publisher) => {
                publisher.send(model);
            }
            EventsLoopMode::Publisher(_) => {
                panic!(
                    "Event loop works in publisher mode. Please use publisher to publish messages"
                )
            }
        }
    }

    fn get_receiver(&mut self) -> tokio::sync::mpsc::UnboundedReceiver<EventsLoopMessage<TModel>> {
        match &mut self.inner.mode {
            EventsLoopMode::Unknown => {
                let (publisher, receiver) =
                    EventsLoopPublisher::new(self.name.clone(), self.logger.clone());

                self.inner.mode = EventsLoopMode::NoExternalPublisher(publisher);
                return receiver;
            }
            EventsLoopMode::NoExternalPublisher(_) => {
                panic!("Event loop is already running in no external publisher mode");
            }
            EventsLoopMode::Publisher(receiver) => {
                if let Some(receiver) = receiver.take() {
                    return receiver;
                } else {
                    panic!("Event loop is already running in external publisher mode");
                }
            }
        }
    }

    pub fn start(&mut self, app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>) {
        let receiver = self.get_receiver();

        let event_loop = self.inner.events_loop_tick.take();
        if event_loop.is_none() {
            panic!("Event Loop is not registered");
        }

        let event_loop = event_loop.unwrap();

        let logger = self.logger.clone();
        tokio::spawn(super::event_loop_reader::events_loop_reader(
            self.name.clone(),
            event_loop,
            app_states,
            logger,
            self.iteration_timeout,
            receiver,
        ));
    }

    pub fn stop(&self) {
        match &self.inner.mode {
            EventsLoopMode::Unknown => {
                panic!("Event loop is not running")
            }
            EventsLoopMode::NoExternalPublisher(publisher) => {
                publisher.stop();
            }
            EventsLoopMode::Publisher(_) => {
                panic!(
                    "Event loop works in publisher mode. Please use publisher to publish messages"
                )
            }
        }
    }
}
