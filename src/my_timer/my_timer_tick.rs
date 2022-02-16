#[async_trait::async_trait]
pub trait MyTimerTick {
    async fn tick(&self);
}
