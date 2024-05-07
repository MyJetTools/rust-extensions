use std::{sync::Arc, time::Duration};

use tokio::sync::Mutex;

use crate::{ApplicationStates, Logger, StrOrString};

use super::{EventsLoop, EventsLoopPublisher, EventsLoopTick};

pub struct EventsLoopMutexWrapped<TModel: Send + Sync + 'static> {
    registration_mode: Option<EventsLoop<TModel>>,
    inner: Mutex<Option<EventsLoop<TModel>>>,
    publisher: EventsLoopPublisher<TModel>,
    name: String,
}

impl<TModel: Send + Sync + 'static> EventsLoopMutexWrapped<TModel> {
    pub fn new(
        name: impl Into<StrOrString<'static>>,
        logger: Arc<dyn Logger + Send + Sync + 'static>,
    ) -> Self {
        let name: String = name.into().to_string();

        let mut events_loop = EventsLoop::new(name.clone(), logger);

        let publisher = events_loop.get_publisher();
        Self {
            name: name.to_string(),
            registration_mode: events_loop.into(),
            inner: Mutex::new(None),
            publisher,
        }
    }

    fn get_registration_item(&mut self) -> EventsLoop<TModel> {
        let item = self.registration_mode.take();

        if item.is_none() {
            panic!("Event loop is not on registration mode. Looks like it's already engaged to be working");
        }

        item.unwrap()
    }

    pub fn set_iteration_timeout(mut self, timeout: Duration) {
        let mut item = self.get_registration_item();

        item = item.set_iteration_timeout(timeout);

        self.registration_mode = Some(item);
    }

    pub async fn register_event_loop(
        &self,
        event_loop: Arc<dyn EventsLoopTick<TModel> + Send + Sync + 'static>,
    ) {
        todo!("Implement this")
        /*
        item.register_event_loop(event_loop);

        let mut write_access = self.inner.lock().await;
        *write_access = Some(item);

         */
    }

    pub async fn start(&self, app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>) {
        let mut write_access = self.inner.lock().await;

        if write_access.is_none() {
            panic!("Please register EventLoopTick Before start EventLoop");
        }
        write_access.as_mut().unwrap().start(app_states);
    }

    pub fn send(&self, model: TModel) {
        /*
        match self.publisher.as_ref() {
            Some(sender) => {
                sender.send(model);
            }
            None => {
                panic!(
                    "Sending event to event_loop {} without registering handle ",
                    self.name
                )
            }
        }
         */
    }
}
