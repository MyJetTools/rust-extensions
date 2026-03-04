use std::sync::Arc;

use super::{EventsLoopMode, EventsLoopTick};

pub struct EventsLoopInner<TModel: Send + Sync + 'static> {
    pub events_loop_tick: Option<Arc<dyn EventsLoopTick<TModel> + Send + Sync + 'static>>,
    pub mode: EventsLoopMode<TModel>,
}

impl<TModel: Send + Sync + 'static> EventsLoopInner<TModel> {
    pub fn new() -> Self {
        Self {
            events_loop_tick: None,
            mode: EventsLoopMode::Unknown,
        }
    }
}
