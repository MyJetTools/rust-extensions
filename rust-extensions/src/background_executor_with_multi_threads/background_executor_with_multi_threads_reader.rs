use std::{hash::Hash, panic::AssertUnwindSafe, sync::Arc};

use futures::FutureExt;

use crate::background_executor::RepeatIteration;

use super::background_executor_with_multi_threads::BackgroundExecutorWithMultiThreadsInner;

/// The reader of a single thread id. It is spawned by the trigger which created
/// the thread and it is the only one who removes the thread - so at any moment
/// there is at most one reader per thread id alive.
pub async fn background_executor_with_multi_threads_reader<TThreadId>(
    inner: Arc<BackgroundExecutorWithMultiThreadsInner<TThreadId>>,
    thread_id: TThreadId,
) where
    TThreadId: Hash + Eq + Clone + Send + Sync + 'static,
{
    loop {
        let result = AssertUnwindSafe(inner.job.execute(&thread_id))
            .catch_unwind()
            .await;

        match result {
            Ok(RepeatIteration::Yes) => {
                // The job left the iteration on purpose and asked for another one.
                // The trigger it was serving is not consumed - we go for a new
                // iteration without touching the counter of the thread.
                continue;
            }
            Ok(RepeatIteration::No) => {}
            Err(_) => {
                inner.logger.write_error(
                    format!("BackgroundExecutorWithMultiThreads {}", inner.name.as_str()),
                    "Job is panicked".to_string(),
                    None.into(),
                );
                // A panicked job told us nothing - we consume the trigger, so a
                // job which panics every time can not spin the reader forever.
            }
        }

        if inner.consume_trigger(&thread_id) {
            break;
        }
    }
}
