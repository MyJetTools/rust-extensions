#[async_trait::async_trait]
pub trait RoundTripCallbackWithConfirmation<TItem, TResult, TError> {
    async fn handle(&self, items: &[TItem]) -> Result<Vec<TResult>, TError>;
}
