#[async_trait::async_trait]
pub trait BackgroundJob: Send + Sync + 'static {
    async fn execute(&self);
}
