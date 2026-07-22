/// The actual work which sits behind an idempotency key.
///
/// [`IdempotencyCache`](super::IdempotencyCache) guarantees that for a given key this
/// is executed **at most once** while the key is still remembered - concurrent retries
/// park on the first execution, and later retries get the memorized result.
#[async_trait::async_trait]
pub trait IdempotencyExecution<
    TParams: Send + Sync + 'static,
    TOk: Send + Sync + 'static,
    TErr: Send + Sync + 'static,
>: Send + Sync + 'static
{
    async fn execute(&self, params: TParams) -> Result<TOk, TErr>;
}
