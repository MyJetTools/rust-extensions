use std::{
    panic::AssertUnwindSafe,
    sync::{atomic::Ordering, Arc},
};

use futures::FutureExt;

use super::{background_executor::BackgroundExecutorInner, RepeatIteration};

pub async fn background_executor_reader(inner: Arc<BackgroundExecutorInner>) {
    loop {
        let result = AssertUnwindSafe(inner.job.execute()).catch_unwind().await;

        match result {
            Ok(RepeatIteration::Yes) => {
                // The job left the iteration on purpose and asked for another one.
                // The trigger it was serving is not consumed - we go for a new
                // iteration without touching the counter.
                continue;
            }
            Ok(RepeatIteration::No) => {}
            Err(_) => {
                inner.logger.write_error(
                    format!("BackgroundExecutor {}", inner.name.as_str()),
                    "Job is panicked".to_string(),
                    None.into(),
                );
                // A panicked job told us nothing - we consume the trigger, so a
                // job which panics every time can not spin the reader forever.
            }
        }

        let prev = inner.counter.fetch_sub(1, Ordering::SeqCst);
        if prev == 1 {
            break;
        }
    }
}
