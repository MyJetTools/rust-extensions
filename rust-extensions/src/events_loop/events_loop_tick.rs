pub use super::RepeatIteration;

/// The callback of the [`super::EventsLoop`].
///
/// `tick()` gets the event **by ownership**: the model is moved into the tick,
/// so nothing is cloned and `TModel` does not have to be `Sync`. An iteration
/// which is not done with the event gives it back - `RepeatIteration::Yes(model)`
/// - and the loop hands the very same model to the next iteration.
///
/// A panic or a timeout kills the running future together with the model it
/// owns, so such an event can not be repeated - it is logged and dropped.
#[async_trait::async_trait]
pub trait EventsLoopTick<TModel: Send + 'static>: Send + 'static {
    async fn started(&self);
    /// Handles one event.
    ///
    /// Returns `RepeatIteration::No` when the event is served and the loop may
    /// go for the next one, or `RepeatIteration::Yes(model)` to be started
    /// again with that very model - with a fresh `iteration_timeout` window.
    async fn tick(&self, model: TModel) -> RepeatIteration<TModel>;
    async fn finished(&self);
}
