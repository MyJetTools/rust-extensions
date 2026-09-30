/// The actual work which sits behind a process id.
///
/// [`IdempotencyCache`](super::IdempotencyCache) guarantees that for a given process id
/// this is executed **at most once** while the process id is still remembered - concurrent
/// retries park on the first execution, and later retries get the memorized result.
///
/// The process id is handed in by reference, so the execution can use it (store it next to
/// the side effect, log it) without the caller having to put a copy of it into `params`.
#[async_trait::async_trait]
pub trait IdempotencyExecution<
    TProcessId: Send + Sync + 'static,
    TParams: Send + Sync + 'static,
    TOk: Send + Sync + 'static,
    TErr: Send + Sync + 'static,
>: Send + Sync + 'static
{
    async fn execute(&self, process_id: &TProcessId, params: TParams) -> Result<TOk, TErr>;
}
