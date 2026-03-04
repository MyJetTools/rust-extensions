use super::{events_loop::EventsLoopMessage, EventsLoopPublisher};

pub enum EventsLoopMode<TModel: Send + Sync + 'static> {
    Unknown,
    NoExternalPublisher(EventsLoopPublisher<TModel>),
    Publisher(Option<tokio::sync::mpsc::UnboundedReceiver<EventsLoopMessage<TModel>>>),
}
