use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, OnceLock},
};

use parking_lot::Mutex;

use crate::{Logger, StrOrString};

use super::BackgroundJobWithMultiThreads;

pub(super) struct BackgroundExecutorWithMultiThreadsInner<TThreadId>
where
    TThreadId: Hash + Eq + Clone + Send + Sync + 'static,
{
    /// Amount of not served triggers per thread id. A thread id is present here
    /// only while its reader is alive - the reader which drains the counter
    /// removes the thread id and exits.
    pub threads: Mutex<HashMap<TThreadId, i64>>,
    pub job: Arc<dyn BackgroundJobWithMultiThreads<TThreadId> + Send + Sync + 'static>,
    pub logger: Arc<dyn Logger + Send + Sync + 'static>,
    pub name: Arc<String>,
    /// Where the reader of a thread id is spawned. Captured once, by `start`, so
    /// `trigger` can spawn one without a runtime around the calling thread.
    pub runtime: tokio::runtime::Handle,
}

impl<TThreadId> BackgroundExecutorWithMultiThreadsInner<TThreadId>
where
    TThreadId: Hash + Eq + Clone + Send + Sync + 'static,
{
    /// Consumes the trigger which has just been served.
    ///
    /// Returns `true` when the thread is drained - the thread id is removed and
    /// the reader must exit. The next trigger of this thread id spawns a fresh
    /// reader.
    pub fn consume_trigger(&self, thread_id: &TThreadId) -> bool {
        let mut threads = self.threads.lock();

        let Some(counter) = threads.get_mut(thread_id) else {
            // Can not happen - the thread id is removed by its own reader only.
            return true;
        };

        *counter -= 1;

        if *counter > 0 {
            return false;
        }

        threads.remove(thread_id);
        true
    }
}

/// The same as `BackgroundExecutor`, but the work is split into independent
/// threads by the `thread_id` given to `trigger()`.
///
/// Triggers of the same thread id are served strictly one by one by the single
/// reader task of that thread id; triggers of different thread ids are served
/// in parallel. A reader is spawned by the trigger which created the thread and
/// it lives only while its thread has not served triggers - as soon as they are
/// drained the thread id is removed and the reader task is gone. Nothing is kept
/// alive for an idle thread id, which is what makes an unbounded set of thread
/// ids (an account id, an instrument) affordable.
///
/// **`trigger` is callable from any thread** - a Tokio one, a plain
/// `std::thread`, or an OS thread owned by a C++ host calling in through FFI.
/// The reader is spawned on the runtime captured by `start`, not on the one of
/// the caller. Only `start` has to be called from inside a runtime.
pub struct BackgroundExecutorWithMultiThreads<TThreadId>
where
    TThreadId: Hash + Eq + Clone + Send + Sync + 'static,
{
    job: OnceLock<Arc<dyn BackgroundJobWithMultiThreads<TThreadId> + Send + Sync + 'static>>,
    /// Present exactly once `start` has run - which is also how `trigger` knows
    /// the executor is started, without a flag and without a lock.
    inner: OnceLock<Arc<BackgroundExecutorWithMultiThreadsInner<TThreadId>>>,
    name: Arc<String>,
}

impl<TThreadId> BackgroundExecutorWithMultiThreads<TThreadId>
where
    TThreadId: Hash + Eq + Clone + Send + Sync + 'static,
{
    pub fn new(name: impl Into<StrOrString<'static>>) -> Self {
        let name: Arc<String> = Arc::new(name.into().to_string());

        Self {
            job: OnceLock::new(),
            inner: OnceLock::new(),
            name,
        }
    }

    pub fn register(
        &self,
        job: Arc<dyn BackgroundJobWithMultiThreads<TThreadId> + Send + Sync + 'static>,
    ) {
        if self.job.set(job).is_err() {
            panic!(
                "Background job is already registered for background executor {}",
                self.name
            );
        }
    }

    /// Remembers the runtime the readers are to be spawned on. Must be called
    /// from inside a Tokio runtime - that is the whole reason `trigger` does not
    /// have to be.
    pub fn start(&self, logger: Arc<dyn Logger + Send + Sync + 'static>) {
        let Some(job) = self.job.get() else {
            panic!("Background executor {} is not registered.", self.name);
        };

        let inner = Arc::new(BackgroundExecutorWithMultiThreadsInner {
            threads: Mutex::new(HashMap::new()),
            job: job.clone(),
            logger,
            name: self.name.clone(),
            runtime: tokio::runtime::Handle::current(),
        });

        if self.inner.set(inner).is_err() {
            panic!("Background executor {} is already started.", self.name);
        }
    }

    /// Signals that there may be work to do within the given thread id.
    ///
    /// The very first trigger of a thread id spawns the reader of that thread
    /// id; while the reader is alive the trigger only bumps the counter of the
    /// thread.
    ///
    /// **Callable from any thread**, with or without a Tokio runtime around it:
    /// the reader lands on the runtime captured by `start`, not on the one of
    /// the caller.
    pub fn trigger(&self, thread_id: TThreadId) {
        let Some(inner) = self.inner.get() else {
            panic!("Background executor {} is not started.", self.name);
        };

        let mut threads = inner.threads.lock();

        if let Some(counter) = threads.get_mut(&thread_id) {
            // The reader of this thread id is alive - it will pick the trigger up.
            *counter += 1;
            return;
        }

        threads.insert(thread_id.clone(), 1);
        drop(threads);

        inner.runtime.spawn(
            super::background_executor_with_multi_threads_reader::background_executor_with_multi_threads_reader(
                inner.clone(), thread_id,
            ),
        );
    }

    /// Amount of thread ids which have a reader alive right now.
    pub fn get_working_threads_amount(&self) -> usize {
        match self.inner.get() {
            Some(inner) => inner.threads.lock().len(),
            None => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use parking_lot::Mutex;

    use crate::background_executor::RepeatIteration;
    use crate::Logger;

    use super::{BackgroundExecutorWithMultiThreads, BackgroundJobWithMultiThreads};

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

    #[derive(Default)]
    struct TestState {
        total_runs: AtomicUsize,
        runs_per_thread: Mutex<HashMap<u64, usize>>,
        in_flight_per_thread: Mutex<HashMap<u64, usize>>,
        parallel_now: AtomicUsize,
        max_parallel: AtomicUsize,
        /// A panic inside the job is caught and logged by the reader, so the
        /// single consumer violation is reported through the flag - not by a
        /// panicking assert which nobody would see.
        same_thread_in_parallel: AtomicBool,
    }

    impl TestState {
        fn enter(&self, thread_id: u64) {
            {
                let mut in_flight = self.in_flight_per_thread.lock();
                let in_flight = in_flight.entry(thread_id).or_insert(0);
                *in_flight += 1;
                if *in_flight > 1 {
                    self.same_thread_in_parallel.store(true, Ordering::SeqCst);
                }
            }

            let parallel_now = self.parallel_now.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_parallel.fetch_max(parallel_now, Ordering::SeqCst);
        }

        fn leave(&self, thread_id: u64) {
            self.parallel_now.fetch_sub(1, Ordering::SeqCst);
            *self.in_flight_per_thread.lock().entry(thread_id).or_insert(1) -= 1;
            *self.runs_per_thread.lock().entry(thread_id).or_insert(0) += 1;
            self.total_runs.fetch_add(1, Ordering::SeqCst);
        }

        fn runs_of(&self, thread_id: u64) -> usize {
            self.runs_per_thread
                .lock()
                .get(&thread_id)
                .copied()
                .unwrap_or(0)
        }
    }

    struct CountingJob {
        state: Arc<TestState>,
    }

    #[async_trait::async_trait]
    impl BackgroundJobWithMultiThreads<u64> for CountingJob {
        async fn execute(&self, thread_id: &u64) -> RepeatIteration {
            self.state.enter(*thread_id);
            tokio::time::sleep(Duration::from_millis(1)).await;
            self.state.leave(*thread_id);
            RepeatIteration::No
        }
    }

    /// Asks for `repeats` extra iterations - as a job which left the iteration
    /// early would - and then finishes the trigger it is serving.
    struct RepeatingJob {
        state: Arc<TestState>,
        repeats_left: Mutex<HashMap<u64, usize>>,
    }

    #[async_trait::async_trait]
    impl BackgroundJobWithMultiThreads<u64> for RepeatingJob {
        async fn execute(&self, thread_id: &u64) -> RepeatIteration {
            self.state.enter(*thread_id);
            tokio::time::sleep(Duration::from_millis(1)).await;
            self.state.leave(*thread_id);

            let mut repeats_left = self.repeats_left.lock();
            let left = repeats_left.entry(*thread_id).or_insert(0);

            if *left > 0 {
                *left -= 1;
                return RepeatIteration::Yes;
            }

            RepeatIteration::No
        }
    }

    struct PanickingJob {
        state: Arc<TestState>,
    }

    #[async_trait::async_trait]
    impl BackgroundJobWithMultiThreads<u64> for PanickingJob {
        async fn execute(&self, thread_id: &u64) -> RepeatIteration {
            self.state.enter(*thread_id);
            tokio::time::sleep(Duration::from_millis(1)).await;
            self.state.leave(*thread_id);
            panic!("Job of thread {} is panicked", thread_id);
        }
    }

    fn make_executor(
        state: &Arc<TestState>,
        name: &'static str,
    ) -> Arc<BackgroundExecutorWithMultiThreads<u64>> {
        let executor = Arc::new(BackgroundExecutorWithMultiThreads::new(name));
        executor.register(Arc::new(CountingJob {
            state: state.clone(),
        }));
        executor.start(Arc::new(TestLogger));
        executor
    }

    async fn wait_for(state: &Arc<TestState>, expected: usize) {
        for _ in 0..2000 {
            if state.total_runs.load(Ordering::SeqCst) >= expected {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!(
            "Expected {} runs, got {}",
            expected,
            state.total_runs.load(Ordering::SeqCst)
        );
    }

    async fn wait_until_no_working_threads(executor: &Arc<BackgroundExecutorWithMultiThreads<u64>>) {
        for _ in 0..2000 {
            if executor.get_working_threads_amount() == 0 {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!(
            "Expected no working threads, got {}",
            executor.get_working_threads_amount()
        );
    }

    #[test]
    fn runs_count_equals_trigger_count_per_thread() {
        rt().block_on(async {
            let state = Arc::new(TestState::default());
            let executor = make_executor(&state, "test-per-thread");

            const THREADS: u64 = 5;
            const PER_THREAD: usize = 40;

            for _ in 0..PER_THREAD {
                for thread_id in 0..THREADS {
                    executor.trigger(thread_id);
                }
            }

            let expected = THREADS as usize * PER_THREAD;
            wait_for(&state, expected).await;
            assert_eq!(state.total_runs.load(Ordering::SeqCst), expected);

            for thread_id in 0..THREADS {
                assert_eq!(state.runs_of(thread_id), PER_THREAD);
            }

            assert!(!state.same_thread_in_parallel.load(Ordering::SeqCst));
        });
    }

    #[test]
    fn same_thread_id_is_served_one_by_one() {
        rt().block_on(async {
            let state = Arc::new(TestState::default());
            let executor = make_executor(&state, "test-single-thread");

            const N: usize = 200;
            for _ in 0..N {
                executor.trigger(42);
            }

            wait_for(&state, N).await;
            assert_eq!(state.runs_of(42), N);
            assert!(!state.same_thread_in_parallel.load(Ordering::SeqCst));
            // The one and only reader of the thread 42 was alive the whole time.
            assert_eq!(state.max_parallel.load(Ordering::SeqCst), 1);
        });
    }

    #[test]
    fn different_thread_ids_are_served_in_parallel() {
        rt().block_on(async {
            let state = Arc::new(TestState::default());
            let executor = make_executor(&state, "test-parallel-threads");

            const THREADS: u64 = 8;
            for thread_id in 0..THREADS {
                executor.trigger(thread_id);
            }

            wait_for(&state, THREADS as usize).await;
            assert_eq!(
                state.max_parallel.load(Ordering::SeqCst),
                THREADS as usize,
                "every thread id has to be served by its own reader"
            );
        });
    }

    #[test]
    fn triggers_from_many_tasks_are_not_lost() {
        rt().block_on(async {
            let state = Arc::new(TestState::default());
            let executor = make_executor(&state, "test-many-producers");

            const TASKS: u64 = 10;
            const PER_TASK: usize = 30;

            let mut handles = Vec::new();
            for task_no in 0..TASKS {
                let executor = executor.clone();
                handles.push(tokio::spawn(async move {
                    for _ in 0..PER_TASK {
                        // Producers overlap on thread ids on purpose.
                        executor.trigger(task_no % 3);
                        tokio::task::yield_now().await;
                    }
                }));
            }

            for handle in handles {
                handle.await.unwrap();
            }

            let expected = TASKS as usize * PER_TASK;
            wait_for(&state, expected).await;
            assert_eq!(state.total_runs.load(Ordering::SeqCst), expected);
            assert!(!state.same_thread_in_parallel.load(Ordering::SeqCst));
        });
    }

    #[test]
    fn thread_is_removed_when_drained_and_respawned_on_the_next_trigger() {
        rt().block_on(async {
            let state = Arc::new(TestState::default());
            let executor = make_executor(&state, "test-thread-lifecycle");

            executor.trigger(1);
            executor.trigger(2);
            assert_eq!(executor.get_working_threads_amount(), 2);

            wait_for(&state, 2).await;
            wait_until_no_working_threads(&executor).await;

            // Both readers exited - a fresh trigger has to spawn a new one.
            executor.trigger(1);
            assert_eq!(executor.get_working_threads_amount(), 1);

            wait_for(&state, 3).await;
            wait_until_no_working_threads(&executor).await;

            assert_eq!(state.runs_of(1), 2);
            assert_eq!(state.runs_of(2), 1);
        });
    }

    /// Same FFI story as of the single threaded executor - and the thread ids
    /// still have to be released once they are drained, so that an unbounded set
    /// of them stays affordable.
    #[test]
    fn triggered_from_a_thread_without_a_runtime() {
        rt().block_on(async {
            let state = Arc::new(TestState::default());
            let executor = make_executor(&state, "test-foreign-thread");

            let foreign_thread = {
                let executor = executor.clone();
                std::thread::spawn(move || {
                    assert!(
                        tokio::runtime::Handle::try_current().is_err(),
                        "the test is meaningless if the thread is inside a runtime"
                    );

                    executor.trigger(1);
                    executor.trigger(2);
                })
            };

            wait_for(&state, 2).await;
            foreign_thread.join().unwrap();

            wait_until_no_working_threads(&executor).await;

            // Neither thread id is wedged - a later trigger of the very same id
            // spawns a fresh reader and is served.
            executor.trigger(1);
            wait_for(&state, 3).await;

            assert_eq!(state.runs_of(1), 2);
            assert_eq!(state.runs_of(2), 1);
        });
    }

    #[test]
    fn trigger_before_start_panics_but_does_not_wedge_executor() {
        rt().block_on(async {
            let state = Arc::new(TestState::default());
            let executor = Arc::new(BackgroundExecutorWithMultiThreads::new(
                "test-early-trigger",
            ));

            let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                executor.trigger(1);
            }));
            assert!(panicked.is_err());

            executor.register(Arc::new(CountingJob {
                state: state.clone(),
            }));
            executor.start(Arc::new(TestLogger));

            executor.trigger(1);
            wait_for(&state, 1).await;
            assert_eq!(state.runs_of(1), 1);
        });
    }

    #[test]
    fn repeat_iteration_yes_runs_again_within_the_same_thread() {
        rt().block_on(async {
            let state = Arc::new(TestState::default());
            let executor = Arc::new(BackgroundExecutorWithMultiThreads::new("test-repeat"));

            let mut repeats_left = HashMap::new();
            repeats_left.insert(1, 3);
            repeats_left.insert(2, 1);

            executor.register(Arc::new(RepeatingJob {
                state: state.clone(),
                repeats_left: Mutex::new(repeats_left),
            }));
            executor.start(Arc::new(TestLogger));

            // 2 triggers of the thread 1 + 3 repeats of it, 1 trigger of the
            // thread 2 + 1 repeat of it.
            executor.trigger(1);
            executor.trigger(1);
            executor.trigger(2);

            wait_for(&state, 7).await;
            tokio::time::sleep(Duration::from_millis(50)).await;

            assert_eq!(state.runs_of(1), 5);
            assert_eq!(state.runs_of(2), 2);
            assert!(!state.same_thread_in_parallel.load(Ordering::SeqCst));
        });
    }

    #[test]
    fn panicking_job_consumes_the_trigger_and_releases_the_thread() {
        rt().block_on(async {
            let state = Arc::new(TestState::default());
            let executor = Arc::new(BackgroundExecutorWithMultiThreads::new("test-panic"));
            executor.register(Arc::new(PanickingJob {
                state: state.clone(),
            }));
            executor.start(Arc::new(TestLogger));

            executor.trigger(1);
            executor.trigger(1);

            wait_for(&state, 2).await;
            wait_until_no_working_threads(&executor).await;

            tokio::time::sleep(Duration::from_millis(50)).await;
            assert_eq!(state.runs_of(1), 2);
        });
    }
}
