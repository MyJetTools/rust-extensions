use std::sync::{
    atomic::{AtomicBool, AtomicI64, Ordering},
    Arc,
};

use parking_lot::Mutex;

use crate::{Logger, StrOrString};

use super::BackgroundJob;

pub(super) struct BackgroundExecutorInner {
    pub counter: Arc<AtomicI64>,
    pub job: Arc<dyn BackgroundJob + Send + Sync + 'static>,
    pub logger: Arc<dyn Logger + Send + Sync + 'static>,
    pub name: Arc<String>,
}

pub struct BackgroundExecutor {
    counter: Arc<AtomicI64>,
    pending_job: Mutex<Option<Arc<dyn BackgroundJob + Send + Sync + 'static>>>,
    inner: Mutex<Option<Arc<BackgroundExecutorInner>>>,
    started: AtomicBool,
    name: Arc<String>,
}

impl BackgroundExecutor {
    pub fn new(name: impl Into<StrOrString<'static>>) -> Self {
        let name: Arc<String> = Arc::new(name.into().to_string());

        Self {
            counter: Arc::new(AtomicI64::new(0)),
            pending_job: Mutex::new(None),
            inner: Mutex::new(None),
            started: AtomicBool::new(false),
            name,
        }
    }

    pub fn register(&self, job: Arc<dyn BackgroundJob + Send + Sync + 'static>) {
        let mut pending_job = self.pending_job.lock();

        if pending_job.is_some() {
            panic!(
                "Background job is already registered for background executor {}",
                self.name
            );
        }

        *pending_job = Some(job);
    }

    pub fn start(&self, logger: Arc<dyn Logger + Send + Sync + 'static>) {
        let job = self.pending_job.lock().take();

        let Some(job) = job else {
            panic!(
                "Background executor {} is not registered or already started.",
                self.name
            );
        };

        let inner = Arc::new(BackgroundExecutorInner {
            counter: self.counter.clone(),
            job,
            logger,
            name: self.name.clone(),
        });

        *self.inner.lock() = Some(inner);
        self.started.store(true, Ordering::SeqCst);
    }

    pub fn trigger(&self) {
        // Checked before touching the counter: a leftover increment from a
        // not-started panic would prevent the reader from ever being spawned.
        if !self.started.load(Ordering::SeqCst) {
            panic!("Background executor {} is not started.", self.name);
        }

        let prev = self.counter.fetch_add(1, Ordering::SeqCst);
        if prev == 0 {
            let inner = self.inner.lock();

            let Some(inner) = inner.as_ref() else {
                panic!("Background executor {} is not started.", self.name);
            };

            tokio::spawn(
                super::background_executor_reader::background_executor_reader(inner.clone()),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use crate::Logger;

    use super::{BackgroundExecutor, BackgroundJob};

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
    }

    struct TestLogger;

    impl Logger for TestLogger {
        fn write_info(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
        fn write_warning(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
        fn write_error(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
        fn write_fatal_error(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
        fn write_debug_info(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
    }

    struct CountingJob {
        runs: Arc<AtomicUsize>,
        in_flight: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl BackgroundJob for CountingJob {
        async fn execute(&self) {
            // No two jobs may run at the same time (single consumer invariant).
            let in_flight = self.in_flight.fetch_add(1, Ordering::SeqCst);
            assert_eq!(in_flight, 0, "two jobs executed in parallel");
            tokio::time::sleep(Duration::from_millis(1)).await;
            self.runs.fetch_add(1, Ordering::SeqCst);
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
        }
    }

    fn make_executor(runs: &Arc<AtomicUsize>, name: &'static str) -> Arc<BackgroundExecutor> {
        let executor = Arc::new(BackgroundExecutor::new(name));
        executor.register(Arc::new(CountingJob {
            runs: runs.clone(),
            in_flight: Arc::new(AtomicUsize::new(0)),
        }));
        executor.start(Arc::new(TestLogger));
        executor
    }

    async fn wait_for(runs: &Arc<AtomicUsize>, expected: usize) {
        for _ in 0..2000 {
            if runs.load(Ordering::SeqCst) >= expected {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!(
            "Expected {} runs, got {}",
            expected,
            runs.load(Ordering::SeqCst)
        );
    }

    #[test]
    fn runs_count_equals_trigger_count() {
        rt().block_on(async {
            let runs = Arc::new(AtomicUsize::new(0));
            let executor = make_executor(&runs, "test");

            const N: usize = 200;
            for _ in 0..N {
                executor.trigger();
            }

            wait_for(&runs, N).await;
            assert_eq!(runs.load(Ordering::SeqCst), N);
        });
    }

    #[test]
    fn runs_count_equals_trigger_count_from_many_tasks() {
        rt().block_on(async {
            let runs = Arc::new(AtomicUsize::new(0));
            let executor = make_executor(&runs, "test-parallel");

            const TASKS: usize = 10;
            const PER_TASK: usize = 50;

            let mut handles = Vec::new();
            for _ in 0..TASKS {
                let executor = executor.clone();
                handles.push(tokio::spawn(async move {
                    for _ in 0..PER_TASK {
                        executor.trigger();
                        tokio::task::yield_now().await;
                    }
                }));
            }

            for handle in handles {
                handle.await.unwrap();
            }

            let expected = TASKS * PER_TASK;
            wait_for(&runs, expected).await;
            assert_eq!(runs.load(Ordering::SeqCst), expected);
        });
    }

    #[test]
    fn trigger_before_start_panics_but_does_not_wedge_executor() {
        rt().block_on(async {
            let runs = Arc::new(AtomicUsize::new(0));
            let executor = Arc::new(BackgroundExecutor::new("test-early-trigger"));

            let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                executor.trigger();
            }));
            assert!(panicked.is_err());

            executor.register(Arc::new(CountingJob {
                runs: runs.clone(),
                in_flight: Arc::new(AtomicUsize::new(0)),
            }));
            executor.start(Arc::new(TestLogger));

            executor.trigger();
            wait_for(&runs, 1).await;
            assert_eq!(runs.load(Ordering::SeqCst), 1);
        });
    }

    #[test]
    fn reader_restarts_after_counter_drained() {
        rt().block_on(async {
            let runs = Arc::new(AtomicUsize::new(0));
            let executor = make_executor(&runs, "test-restart");

            executor.trigger();
            wait_for(&runs, 1).await;

            // counter is drained back to 0 here; the next trigger must spawn a fresh reader
            executor.trigger();
            wait_for(&runs, 2).await;
            assert_eq!(runs.load(Ordering::SeqCst), 2);
        });
    }
}
