pub use crate::background_executor::RepeatIteration;

/// The callback of the [`super::EventsLoop`].
///
/// `tick()` gets the event **by reference** on purpose: the event stays owned by
/// the loop reader, so it is still there after the tick returned
/// `RepeatIteration::Yes`, after the tick panicked and after it was timed out -
/// and the very same event can be given to the tick again.
///
/// `TModel` has to be `Sync` for that - the tick holds a `&TModel` across its
/// awaits. A model which can not be shared has to be wrapped into something
/// which can (`Mutex`, `Arc`, ...).
#[async_trait::async_trait]
pub trait EventsLoopTick<TModel: Send + Sync + 'static>: Send + 'static {
    async fn started(&self);
    /// Handles one event.
    ///
    /// Returns `RepeatIteration::No` when the event is served and the loop may
    /// go for the next one, or `RepeatIteration::Yes` to be started again with
    /// the same event - with a fresh `iteration_timeout` window.
    async fn tick(&self, model: &TModel) -> RepeatIteration;
    async fn finished(&self);
}
