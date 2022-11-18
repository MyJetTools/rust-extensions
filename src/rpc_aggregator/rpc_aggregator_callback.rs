#[async_trait::async_trait]
pub trait RpcAggregatorCallback<TItem, TError> {
    async fn handle(&self, items: &[TItem]) -> Result<(), TError>;
}
