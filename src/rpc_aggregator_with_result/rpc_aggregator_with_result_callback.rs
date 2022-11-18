#[async_trait::async_trait]
pub trait RpcAggregatorWithResultCallback<TItem, TResult, TError> {
    async fn handle(&self, items: &[TItem]) -> Result<Vec<TResult>, TError>;
}
