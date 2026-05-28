use std::{
    panic::AssertUnwindSafe,
    sync::{atomic::Ordering, Arc},
};

use futures::FutureExt;

use super::background_executor::BackgroundExecutorInner;

pub async fn background_executor_reader(inner: Arc<BackgroundExecutorInner>) {
    loop {
        let result = AssertUnwindSafe(inner.job.execute()).catch_unwind().await;

        if result.is_err() {
            inner.logger.write_error(
                format!("BackgroundExecutor {}", inner.name.as_str()),
                "Job is panicked".to_string(),
                None.into(),
            );
        }

        let prev = inner.counter.fetch_sub(1, Ordering::SeqCst);
        if prev == 1 {
            break;
        }
    }
}
