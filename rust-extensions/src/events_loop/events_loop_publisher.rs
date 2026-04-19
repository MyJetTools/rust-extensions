use std::sync::Arc;

use super::EventsLoopMessage;


pub struct EventsLoopPublisher<TModel: Send + Sync + 'static> {
    sender: Arc<tokio::sync::mpsc::UnboundedSender<EventsLoopMessage<TModel>>>,
    name: Arc<String>,
}

impl<TModel: Send + Sync + 'static> EventsLoopPublisher<TModel> {
    pub(super) fn new(
        name: Arc<String>,
        sender: tokio::sync::mpsc::UnboundedSender<EventsLoopMessage<TModel>>,
    ) -> Self {
        Self { sender: Arc::new(sender), name }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
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

    pub fn clone(&self)->Self{
        Self { sender: self.sender.clone(), name: self.name.clone() }
    }
}
