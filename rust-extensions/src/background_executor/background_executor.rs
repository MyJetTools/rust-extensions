use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, OnceLock,
};

use tokio::sync::Semaphore;

use crate::{Logger, StrOrString};

use super::BackgroundJob;

/// Everything the reader task needs. It is handed over to the task by `start`
/// and owned by it alone - there is nothing here the producer side reaches back
/// into.
pub(super) struct BackgroundExecutorInner {
    pub triggers: Arc<Semaphore>,
    pub job: Arc<dyn BackgroundJob + Send + Sync + 'static>,
    pub logger: Arc<dyn Logger + Send + Sync + 'static>,
    pub name: Arc<String>,
}

/// Offloads work from the caller onto a single background Tokio task.
///
/// The reader task is spawned once, by `start`, and lives as long as the
/// application does. A trigger is a permit added to a semaphore, so `trigger`
/// spawns nothing, locks nothing and awaits nothing - **it is legal from any
/// thread**: a Tokio one, a plain `std::thread`, or an OS thread owned by a C++
/// host calling in through FFI. Only `start` has to be called from inside a
/// runtime.
///
/// The semaphore is what counts the unserved triggers: permits accumulate, so N
/// triggers are N iterations no matter how they interleave with the reader.
///
/// There is not a single mutex in the whole type - the job is registered into a
/// `OnceLock`, which is also what makes a second `register` an error rather than
/// a silent overwrite.
pub struct BackgroundExecutor {
    triggers: Arc<Semaphore>,
    job: OnceLock<Arc<dyn BackgroundJob + Send + Sync + 'static>>,
    started: AtomicBool,
    name: Arc<String>,
}

impl BackgroundExecutor {
    pub fn new(name: impl Into<StrOrString<'static>>) -> Self {
        let name: Arc<String> = Arc::new(name.into().to_string());

        Self {
            triggers: Arc::new(Semaphore::new(0)),
            job: OnceLock::new(),
            started: AtomicBool::new(false),
            name,
        }
    }

    pub fn register(&self, job: Arc<dyn BackgroundJob + Send + Sync + 'static>) {
        if self.job.set(job).is_err() {
            panic!(
                "Background job is already registered for background executor {}",
                self.name
            );
        }
    }

    /// Spawns the one and only reader task. Must be called from inside a Tokio
    /// runtime - that is the whole reason `trigger` does not have to be.
    pub fn start(&self, logger: Arc<dyn Logger + Send + Sync + 'static>) {
        let Some(job) = self.job.get() else {
            panic!("Background executor {} is not registered.", self.name);
        };

        // Claims the right to start before anything is spawned, so a second
        // `start` can not put a second reader on the same semaphore. `Relaxed`
        // is enough here too - what is needed is the atomicity of the swap, not
        // an ordering against anything else.
        if self
            .started
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            panic!("Background executor {} is already started.", self.name);
        }

        tokio::spawn(super::background_executor_reader::background_executor_reader(
            BackgroundExecutorInner {
                triggers: self.triggers.clone(),
                job: job.clone(),
                logger,
                name: self.name.clone(),
            },
        ));
    }

    /// Signals that there may be work to do.
    ///
    /// **Callable from any thread**, with or without a Tokio runtime around it.
    /// The reader is already running, so this only hands it one more permit -
    /// no lock, no await, no spawn.
    pub fn trigger(&self) {
        // `Relaxed` is enough: the flag carries no happens-before duty. It only
        // catches "triggered before start", and the semaphore - the single thing
        // this method touches - synchronizes itself. A stronger ordering would
        // not even buy a sharper check: it does not make the store of `start`
        // visible any sooner, it only orders it against other operations.
        if !self.started.load(Ordering::Relaxed) {
            panic!("Background executor {} is not started.", self.name);
        }

        self.triggers.add_permits(1);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use crate::background_executor::RepeatIteration;
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
        async fn execute(&self) -> RepeatIteration {
            // No two jobs may run at the same time (single consumer invariant).
            let in_flight = self.in_flight.fetch_add(1, Ordering::SeqCst);
            assert_eq!(in_flight, 0, "two jobs executed in parallel");
            tokio::time::sleep(Duration::from_millis(1)).await;
            self.runs.fetch_add(1, Ordering::SeqCst);
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
            RepeatIteration::No
        }
    }

    /// Asks for `repeats` extra iterations - as a job which left the iteration
    /// early would - and then finishes the trigger it is serving.
    struct RepeatingJob {
        runs: Arc<AtomicUsize>,
        in_flight: Arc<AtomicUsize>,
        repeats_left: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl BackgroundJob for RepeatingJob {
        async fn execute(&self) -> RepeatIteration {
            let in_flight = self.in_flight.fetch_add(1, Ordering::SeqCst);
            assert_eq!(in_flight, 0, "two jobs executed in parallel");
            tokio::time::sleep(Duration::from_millis(1)).await;
            self.runs.fetch_add(1, Ordering::SeqCst);
            self.in_flight.fetch_sub(1, Ordering::SeqCst);

            if self
                .repeats_left
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                    if left == 0 {
                        None
                    } else {
                        Some(left - 1)
                    }
                })
                .is_ok()
            {
                return RepeatIteration::Yes;
            }

            RepeatIteration::No
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

    /// The FFI story: a C++ callback lands on an OS thread we do not own, with
    /// no runtime around it, and the work still has to be done. `trigger` spawns
    /// nothing - the reader is already up - so it does not care where it is
    /// called from.
    #[test]
    fn triggered_from_a_thread_without_a_runtime() {
        rt().block_on(async {
            let runs = Arc::new(AtomicUsize::new(0));
            let executor = make_executor(&runs, "test-foreign-thread");

            let foreign_thread = {
                let executor = executor.clone();
                std::thread::spawn(move || {
                    // No runtime here - not even an entered one. This is what an
                    // FFI callback thread looks like.
                    assert!(
                        tokio::runtime::Handle::try_current().is_err(),
                        "the test is meaningless if the thread is inside a runtime"
                    );

                    executor.trigger();
                    executor.trigger();
                })
            };

            wait_for(&runs, 2).await;
            foreign_thread.join().unwrap();

            // And the executor is not wedged afterwards - a later trigger is
            // served like any other. This is the half which catches a trigger
            // that failed to bring the reader up, rather than one that merely
            // panicked on the foreign thread.
            executor.trigger();
            wait_for(&runs, 3).await;

            tokio::time::sleep(Duration::from_millis(50)).await;
            assert_eq!(runs.load(Ordering::SeqCst), 3);
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
    fn repeat_iteration_yes_runs_again_without_consuming_the_trigger() {
        rt().block_on(async {
            let runs = Arc::new(AtomicUsize::new(0));
            let executor = Arc::new(BackgroundExecutor::new("test-repeat"));
            executor.register(Arc::new(RepeatingJob {
                runs: runs.clone(),
                in_flight: Arc::new(AtomicUsize::new(0)),
                repeats_left: AtomicUsize::new(3),
            }));
            executor.start(Arc::new(TestLogger));

            // A single trigger, but the job asks for 3 extra iterations.
            executor.trigger();
            wait_for(&runs, 4).await;

            // Every permit is used up now, so the reader is parked - a fresh
            // trigger has to wake it and run exactly once.
            executor.trigger();
            wait_for(&runs, 5).await;
            tokio::time::sleep(Duration::from_millis(50)).await;
            assert_eq!(runs.load(Ordering::SeqCst), 5);
        });
    }

    #[test]
    fn repeats_do_not_swallow_the_triggers_arrived_meanwhile() {
        rt().block_on(async {
            let runs = Arc::new(AtomicUsize::new(0));
            let executor = Arc::new(BackgroundExecutor::new("test-repeat-and-trigger"));
            executor.register(Arc::new(RepeatingJob {
                runs: runs.clone(),
                in_flight: Arc::new(AtomicUsize::new(0)),
                repeats_left: AtomicUsize::new(2),
            }));
            executor.start(Arc::new(TestLogger));

            // 3 triggers + 2 repeats = 5 iterations, and not one less.
            executor.trigger();
            executor.trigger();
            executor.trigger();

            wait_for(&runs, 5).await;
            tokio::time::sleep(Duration::from_millis(50)).await;
            assert_eq!(runs.load(Ordering::SeqCst), 5);
        });
    }

    #[test]
    fn triggers_separated_by_an_idle_period_are_served() {
        rt().block_on(async {
            let runs = Arc::new(AtomicUsize::new(0));
            let executor = make_executor(&runs, "test-restart");

            executor.trigger();
            wait_for(&runs, 1).await;

            // no permits left here, the reader is parked; the next trigger must wake it
            executor.trigger();
            wait_for(&runs, 2).await;
            assert_eq!(runs.load(Ordering::SeqCst), 2);
        });
    }
}
