use std::{sync::Arc, time::Duration};

use tokio::sync::Mutex;

use crate::{ApplicationStates, Logger, StrOrString};

use super::{events_loop::EventsLoopMessage, EventsLoopTick};

struct EventsLoopInner<TModel: Send + Sync + 'static> {
    event_loop_tick: Option<Arc<dyn EventsLoopTick<TModel> + Send + Sync + 'static>>,
    receiver: Option<tokio::sync::mpsc::UnboundedReceiver<EventsLoopMessage<TModel>>>,
}

impl<TModel: Send + Sync + 'static> EventsLoopInner<TModel> {
    pub fn new(receiver: tokio::sync::mpsc::UnboundedReceiver<EventsLoopMessage<TModel>>) -> Self {
        Self {
            event_loop_tick: None,
            receiver: Some(receiver),
        }
    }
}

pub struct EventsLoopMutexWrapped<TModel: Send + Sync + 'static> {
    inner: Mutex<EventsLoopInner<TModel>>,
    sender: tokio::sync::mpsc::UnboundedSender<EventsLoopMessage<TModel>>,
    name: Arc<String>,
    iteration_timeout: Duration,
}

impl<TModel: Send + Sync + 'static> EventsLoopMutexWrapped<TModel> {
    pub fn new(name: impl Into<StrOrString<'static>>) -> Self {
        let name: String = name.into().to_string();

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        Self {
            name: Arc::new(name),
            sender,
            iteration_timeout: Duration::from_secs(30),
            inner: Mutex::new(EventsLoopInner::new(receiver)),
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
        let mut write_access = self.inner.lock().await;

        if write_access.event_loop_tick.is_some() {
            panic!(
                "Event loop tick is already registered for this event loop {}",
                self.name
            );
        }

        write_access.event_loop_tick = Some(event_loop);
    }

    pub async fn start(
        &self,
        app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
        logger: Arc<dyn Logger + Send + Sync + 'static>,
    ) {
        let mut write_access = self.inner.lock().await;

        let receiver = write_access.receiver.take();

        if receiver.is_none() {
            panic!("Event Loop {} is already started.", self.name);
        }

        let event_loop_tick = write_access.event_loop_tick.take();

        if event_loop_tick.is_none() {
            panic!(
                "Please register EventLoopTick Before start EventLoop {}",
                self.name
            );
        }

        tokio::spawn(super::event_loop_reader::events_loop_reader(
            self.name.clone(),
            event_loop_tick.unwrap(),
            app_states,
            logger,
            self.iteration_timeout,
            receiver.unwrap(),
        ));
    }

    pub fn send(&self, model: TModel) {
        if let Err(err) = self.sender.send(EventsLoopMessage::NewMessage(model)) {
            panic!(
                "Error while sending message to event loop {}. Err: {}",
                self.name, err
            );
        }
    }

    pub fn stop(&self) {
        if let Err(err) = self.sender.send(EventsLoopMessage::Shutdown) {
            panic!(
                "Error while sending message to event loop {}. Err: {}",
                self.name, err
            );
        }
    }
}
