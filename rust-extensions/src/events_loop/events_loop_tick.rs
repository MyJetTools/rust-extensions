
#[async_trait::async_trait]
pub trait EventsLoopTick<TModel: 'static>: Send + 'static {
    async fn started(&self);
    async fn tick(&self, model: TModel);
    async fn finished(&self);
}
