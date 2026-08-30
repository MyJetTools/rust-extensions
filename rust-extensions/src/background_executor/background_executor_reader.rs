use std::panic::AssertUnwindSafe;

use futures::FutureExt;

use super::{background_executor::BackgroundExecutorInner, RepeatIteration};

/// The single reader task of a [`super::BackgroundExecutor`]. It is spawned once,
/// by `start`, and lives as long as the application does - so a `trigger` has
/// nothing to spawn and needs no runtime of its own.
///
/// One permit of the semaphore is one unserved trigger. Permits accumulate while
/// the reader is busy, so nothing is lost and nothing has to be counted on the
/// side: the semaphore is the queue.
pub async fn background_executor_reader(inner: BackgroundExecutorInner) {
    loop {
        let Ok(permit) = inner.triggers.acquire().await else {
            // The semaphore is never closed, so this can not happen - but if it
            // ever did, there would be no more triggers to wait for.
            return;
        };

        // The trigger is taken for good - the permit is not given back to the
        // semaphore when it goes out of scope.
        permit.forget();

        loop {
            let result = AssertUnwindSafe(inner.job.execute()).catch_unwind().await;

            match result {
                Ok(RepeatIteration::Yes) => {
                    // The job left the iteration on purpose and asked for another
                    // one. The trigger it is serving is not consumed - we go for a
                    // new iteration without taking one more permit.
                    continue;
                }
                Ok(RepeatIteration::No) => break,
                Err(_) => {
                    inner.logger.write_error(
                        format!("BackgroundExecutor {}", inner.name.as_str()),
                        "Job is panicked".to_string(),
                        None.into(),
                    );
                    // A panicked job told us nothing - the trigger stays consumed,
                    // so a job which panics every time can not spin the reader.
                    break;
                }
            }
        }
    }
}
