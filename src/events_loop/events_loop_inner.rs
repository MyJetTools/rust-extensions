use std::sync::Arc;

use super::{events_loop::EventsLoopMessage, EventsLoopTick};

pub struct EventsLoopInner<TModel: Send + Sync + 'static> {
    pub events_loop_tick: Option<Arc<dyn EventsLoopTick<TModel> + Send + Sync + 'static>>,
    pub sender: Option<tokio::sync::mpsc::UnboundedSender<EventsLoopMessage<TModel>>>,
}

impl<TModel: Send + Sync + 'static> EventsLoopInner<TModel> {
    pub fn new() -> Self {
        Self {
            events_loop_tick: None,
            sender: None,
        }
    }
}
