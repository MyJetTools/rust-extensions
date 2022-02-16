#[async_trait::async_trait]
pub trait MyTimerTick {
    fn get_name<'s>(&self) -> &str;
    async fn tick(&self);
}
