#[async_trait::async_trait]
pub trait EventsLoopTick<TModel: Send + Sync + 'static> {
    async fn started(&self);
    async fn tick(&self, model: TModel);
    async fn finished(&self);
}
