use std::sync::Arc;

use crate::Logger;

use super::events_loop::EventsLoopMessage;

pub struct EventsLoopPublisher<TModel: Send + Sync + 'static> {
    name: Arc<String>,
    sender: tokio::sync::mpsc::UnboundedSender<EventsLoopMessage<TModel>>,

    logger: Arc<dyn Logger + Send + Sync + 'static>,
}

impl<TModel: Send + Sync + 'static> EventsLoopPublisher<TModel> {
    pub fn new(
        name: Arc<String>,
        logger: Arc<dyn Logger + Send + Sync + 'static>,
    ) -> (
        Self,
        tokio::sync::mpsc::UnboundedReceiver<EventsLoopMessage<TModel>>,
    ) {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let result = Self {
            name,
            sender,

            logger,
        };

        (result, receiver)
    }

    pub fn send(&self, model: TModel) {
        if let Err(_) = self.sender.send(EventsLoopMessage::NewMessage(model)) {
            println!("Can not send model to event loop {}", self.name.as_str());
        }
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
