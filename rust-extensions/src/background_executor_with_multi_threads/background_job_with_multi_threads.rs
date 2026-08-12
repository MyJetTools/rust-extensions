use crate::background_executor::RepeatIteration;

/// The job which is executed by the [`super::BackgroundExecutorWithMultiThreads`].
///
/// The same instance serves every thread id - the id of the thread the current
/// iteration belongs to is given to `execute()`, and the executor guarantees
/// that two iterations of the same thread id never overlap.
#[async_trait::async_trait]
pub trait BackgroundJobWithMultiThreads<TThreadId>: Send + Sync + 'static
where
    TThreadId: Send + Sync + 'static,
{
    async fn execute(&self, thread_id: &TThreadId) -> RepeatIteration;
}
