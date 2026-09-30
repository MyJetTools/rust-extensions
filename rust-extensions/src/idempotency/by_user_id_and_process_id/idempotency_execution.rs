/// The actual work which sits behind a process id of a user.
///
/// [`IdempotencyCache`](super::IdempotencyCache) guarantees that for a given pair of user
/// id and process id this is executed **at most once** while the pair is still remembered -
/// concurrent retries park on the first execution, and later retries get the memorized
/// result.
///
/// Both ids are handed in by reference, so the execution acts on behalf of `user_id`
/// without the caller having to put copies of the ids into `params`.
#[async_trait::async_trait]
pub trait IdempotencyExecution<
    TUserId: Send + Sync + 'static,
    TProcessId: Send + Sync + 'static,
    TParams: Send + Sync + 'static,
    TOk: Send + Sync + 'static,
    TErr: Send + Sync + 'static,
>: Send + Sync + 'static
{
    async fn execute(
        &self,
        user_id: &TUserId,
        process_id: &TProcessId,
        params: TParams,
    ) -> Result<TOk, TErr>;
}
