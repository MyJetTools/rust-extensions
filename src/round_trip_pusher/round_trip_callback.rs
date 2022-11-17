#[async_trait::async_trait]
pub trait RoundTripCallback<TItem> {
    async fn handle(&self, items: &[TItem]);
}
