use std::{sync::Arc, time::Duration};

use tokio::sync::Mutex;

use crate::{ApplicationStates, Logger, StrOrString};

use super::{EventsLoopPublisher, EventsLoopTick};

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

pub(super) struct EventsLoopInner<TModel: Send + Sync + 'static> {
    pub event_loop_tick: Arc<dyn EventsLoopTick<TModel> + Send + Sync + 'static>,
    pub receiver: tokio::sync::mpsc::UnboundedReceiver<EventsLoopMessage<TModel>>,
}

pub struct EventsLoop<TModel: Send + Sync + 'static> {
    pending_receiver:
        Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<EventsLoopMessage<TModel>>>>,
    inner: Mutex<Option<EventsLoopInner<TModel>>>,
    publisher: Arc<EventsLoopPublisher<TModel>>,
    name: Arc<String>,
    iteration_timeout: Duration,
}

impl<TModel: Send + Sync + 'static> EventsLoop<TModel> {
    pub fn new(name: impl Into<StrOrString<'static>>) -> Self {
        let name: Arc<String> = Arc::new(name.into().to_string());

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        Self {
            publisher: Arc::new(EventsLoopPublisher::new(name.clone(), sender)),
            name,
            iteration_timeout: Duration::from_secs(30),
            pending_receiver: Mutex::new(Some(receiver)),
            inner: Mutex::new(None),
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
        let receiver = self.pending_receiver.lock().await.take();

        if receiver.is_none() {
            panic!(
                "Event loop tick is already registered for this event loop {}",
                self.name
            );
        }

        let mut inner_lock = self.inner.lock().await;
        *inner_lock = Some(EventsLoopInner {
            event_loop_tick: event_loop,
            receiver: receiver.unwrap(),
        });
    }

    pub async fn start(
        &self,
        app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
        logger: Arc<dyn Logger + Send + Sync + 'static>,
    ) {
        let inner = self.inner.lock().await.take();

        if inner.is_none() {
            panic!(
                "Event Loop {} is not registered or already started.",
                self.name
            );
        }

        tokio::spawn(super::event_loop_reader::events_loop_reader(
            self.name.clone(),
            inner.unwrap(),
            app_states,
            logger,
            self.iteration_timeout,
        ));
    }

    pub fn get_publisher(&self) -> Arc<EventsLoopPublisher<TModel>> {
        self.publisher.clone()
    }

    pub fn send(&self, model: TModel) {
        self.publisher.send(model);
    }

    pub fn stop(&self) {
        self.publisher.stop();
    }
}
