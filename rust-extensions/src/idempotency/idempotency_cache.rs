use std::collections::VecDeque;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use parking_lot::Mutex;

use crate::{StrOrString, TaskCompletion};

use super::{IdempotencyCacheItem, IdempotencyEntry, IdempotencyExecution, IdempotencyResult};

/// How many completed results are kept by default.
pub const DEFAULT_MAX_AMOUNT: usize = 1000;

/// How long a single execution is allowed to take by default.
pub const DEFAULT_EXECUTION_TIMEOUT: Duration = Duration::from_secs(5);

type RegisteredExecution<TParams, TOk, TErr> = Arc<dyn IdempotencyExecution<TParams, TOk, TErr>>;

/// A single flat queue: new keys are pushed to the back, the oldest results are dropped
/// from the front. There is deliberately no index next to it - one container means there
/// is no second structure that could drift out of sync with this one.
struct IdempotencyCacheInner<TOk, TErr> {
    items: VecDeque<IdempotencyCacheItem<TOk, TErr>>,
    max_amount: usize,
}

impl<TOk, TErr> IdempotencyCacheInner<TOk, TErr> {
    fn find_index(&self, key: &str) -> Option<usize> {
        self.items.iter().position(|item| item.key == key)
    }

    fn get_completed_amount(&self) -> usize {
        self.items.iter().filter(|item| item.entry.is_completed()).count()
    }

    /// Drops the oldest completed results until we are back within `max_amount`.
    ///
    /// Only `Completed` entries are eviction candidates: an `Executing` one is being
    /// awaited by somebody, and dropping it would panic every one of them. So `max_amount`
    /// caps the memorized answers, and in-flight executions sit on top of that.
    fn gc(&mut self) {
        while self.get_completed_amount() > self.max_amount {
            let Some(index) = self.items.iter().position(|item| item.entry.is_completed())
            else {
                break;
            };

            self.items.remove(index);
        }
    }
}

/// De-duplicates retries of the same request.
///
/// A request is identified by its idempotency key (typically the client's request id).
/// For a given key:
///
/// - **first call** - runs [`IdempotencyExecution::execute`] inline (right inside
///   `execute`) and memorizes its `Result`;
/// - **a retry while the first call is still running** - does not execute anything: it
///   parks on a [`TaskCompletion`] and is released with the very same result;
/// - **a retry after it finished** - gets the memorized result immediately, the
///   execution is not touched.
///
/// Both `Ok` and `Err` are memorized: once a key produced an answer, every retry of that
/// key gets that answer back.
///
/// The last `max_amount` results are kept, evicted **FIFO by completion time** (a cache
/// hit does not refresh the entry). `max_amount == 0` is legal and means "de-duplicate
/// concurrent retries, but do not remember anything afterwards". In-flight executions are
/// never eviction candidates, so they do not count against `max_amount`.
///
/// Lookups are a linear scan over a single queue, which is the right shape at these sizes
/// (a `String` comparison rejects on length first) and keeps the whole state in one
/// container. It is not the right shape for a `max_amount` in the hundreds of thousands.
///
/// It is designed to live inside an `AppCtx` as a plain field - every method takes
/// `&self`, no outer `Mutex` needed.
///
/// # Cancellation, timeouts and panics
///
/// The first caller owns the execution, so it also owns its fate. If that caller's future
/// is dropped (HTTP timeout, cancelled task), or the execution panics, or it runs longer
/// than the execution timeout, the entry is removed, so the next retry starts the execution
/// from scratch, and everybody parked on it gets the standard [`TaskCompletion`] drop
/// behaviour: their `get_result()` panics with `"Task is dropped"`. Nothing is memorized in
/// any of those cases, because we do not know whether the side effect happened.
///
/// The timeout is just the third way to not produce a result, so it is handled as a panic
/// like the other two: the execution future is dropped and the owner panics too. It also
/// bounds how long an `Executing` entry can hold its key - without it a hung execution
/// would pin that key forever and every retry of it would park forever.
/// Default [`DEFAULT_EXECUTION_TIMEOUT`], changed with
/// [`IdempotencyCache::set_execution_timeout`]. It needs a Tokio runtime with time enabled.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use rust_extensions::{IdempotencyCache, IdempotencyExecution, DEFAULT_MAX_AMOUNT};
///
/// pub struct ChargeParams {
///     pub amount: f64,
/// }
///
/// struct ChargeExecution;
///
/// #[async_trait::async_trait]
/// impl IdempotencyExecution<ChargeParams, String, String> for ChargeExecution {
///     async fn execute(&self, params: ChargeParams) -> Result<String, String> {
///         // the real, non-idempotent work happens here exactly once per key
///         Ok(format!("charged {}", params.amount))
///     }
/// }
///
/// pub struct AppCtx {
///     pub charges: IdempotencyCache<ChargeParams, String, String>,
/// }
///
/// # async fn example() {
/// let ctx = AppCtx {
///     charges: IdempotencyCache::new_with_max_amount("charges", DEFAULT_MAX_AMOUNT),
/// };
/// ctx.charges.register_execution(Arc::new(ChargeExecution));
///
/// // Retrying this with the same key never charges twice.
/// let result = ctx
///     .charges
///     .execute("request-id-1".to_string(), ChargeParams { amount: 10.0 })
///     .await;
/// # }
/// ```
pub struct IdempotencyCache<
    TParams: Send + Sync + 'static,
    TOk: Send + Sync + 'static,
    TErr: Send + Sync + 'static,
> {
    inner: Mutex<IdempotencyCacheInner<TOk, TErr>>,
    /// Written once, read on every `execute`. A `OnceLock` rather than a `Mutex` or an
    /// `ArcSwap`: reading it is a single atomic load which hands back a *reference*, so
    /// the hot path never touches the `Arc` refcount at all.
    execution: OnceLock<RegisteredExecution<TParams, TOk, TErr>>,
    execution_timeout: Duration,
    name: Arc<String>,
}

impl<TParams: Send + Sync + 'static, TOk: Send + Sync + 'static, TErr: Send + Sync + 'static>
    IdempotencyCache<TParams, TOk, TErr>
{
    /// Creates a cache which keeps the last [`DEFAULT_MAX_AMOUNT`] results.
    pub fn new(name: impl Into<StrOrString<'static>>) -> Self {
        Self::new_with_max_amount(name, DEFAULT_MAX_AMOUNT)
    }

    /// Creates a cache which keeps the last `max_amount` results.
    pub fn new_with_max_amount(name: impl Into<StrOrString<'static>>, max_amount: usize) -> Self {
        Self {
            inner: Mutex::new(IdempotencyCacheInner {
                items: VecDeque::new(),
                max_amount,
            }),
            execution: OnceLock::new(),
            execution_timeout: DEFAULT_EXECUTION_TIMEOUT,
            name: Arc::new(name.into().to_string()),
        }
    }

    /// Caps how long a single execution may take. Overrunning it is treated exactly like
    /// a panic - see the type documentation. Default [`DEFAULT_EXECUTION_TIMEOUT`].
    ///
    /// Builder style: `IdempotencyCache::new("charges").set_execution_timeout(timeout)`.
    pub fn set_execution_timeout(mut self, execution_timeout: Duration) -> Self {
        self.execution_timeout = execution_timeout;
        self
    }

    /// Registers the execution. One-shot: a second call panics.
    ///
    /// It is a separate step (not a constructor argument) so the execution is free to
    /// hold an `Arc` of the very `AppCtx` which owns this cache.
    pub fn register_execution(&self, execution: RegisteredExecution<TParams, TOk, TErr>) {
        if self.execution.set(execution).is_err() {
            panic!(
                "Execution is already registered for the idempotency cache {}",
                self.name
            );
        }
    }

    /// Borrowed, not cloned - the caller only needs it for the duration of its own
    /// `&self`, so the hot path costs one atomic load and no refcount traffic.
    fn get_execution(&self) -> &dyn IdempotencyExecution<TParams, TOk, TErr> {
        match self.execution.get() {
            Some(execution) => execution.as_ref(),
            None => panic!(
                "Execution is not registered for the idempotency cache {}",
                self.name
            ),
        }
    }

    /// Returns the result of `key`, executing it only if it has to be executed.
    ///
    /// See the type documentation for what happens on a retry, on cancellation and on a
    /// panic. `params` is consumed only by the caller which actually executes; the ones
    /// which get a memorized result simply drop it.
    pub async fn execute(&self, key: String, params: TParams) -> IdempotencyResult<TOk, TErr> {
        // Resolved before we claim the key: panicking here after inserting the `Executing`
        // entry would leave that entry stuck in the queue forever.
        let execution = self.get_execution();

        let awaiter = {
            let mut inner = self.inner.lock();

            match inner.find_index(key.as_str()) {
                Some(index) => match &mut inner.items[index].entry {
                    IdempotencyEntry::Completed(result) => return result.clone(),
                    IdempotencyEntry::Executing(awaiters) => {
                        let mut task_completion = TaskCompletion::new();
                        let awaiter = task_completion.get_awaiter();
                        awaiters.push(task_completion);
                        Some(awaiter)
                    }
                },
                None => {
                    inner.items.push_back(IdempotencyCacheItem {
                        key: key.clone(),
                        entry: IdempotencyEntry::Executing(Vec::new()),
                    });
                    None
                }
            }
        };

        if let Some(awaiter) = awaiter {
            return awaiter.get_result().await;
        }

        // From here on we own the execution of this key. The guard makes sure the
        // `Executing` entry never outlives us: if this future is cancelled or the
        // execution panics, the entry is removed and the parked `TaskCompletion`s are
        // dropped, which makes their awaiters panic with "Task is dropped".
        let mut guard = ExecutionOwnerGuard::new(&self.inner, key);

        // An overrun is the third way to not produce a result, so it is handled like the
        // other two: `timeout` drops the execution future, and the panic unwinds through
        // the guard, which frees the key and releases the awaiters.
        let executed = tokio::time::timeout(self.execution_timeout, execution.execute(params)).await;

        let Ok(executed) = executed else {
            panic!(
                "Idempotency execution of the key '{}' in the cache '{}' timed out after {:?}",
                guard.get_key(),
                self.name,
                self.execution_timeout
            );
        };

        let result = match executed {
            Ok(ok) => Ok(Arc::new(ok)),
            Err(err) => Err(Arc::new(err)),
        };

        let awaiters = guard.commit(result.clone());

        // Outside the lock. `try_*` and not the panicking versions: an awaiter could have
        // been cancelled while we were executing, and then its receiver is already gone.
        for mut awaiter in awaiters {
            let _ = match result.as_ref() {
                Ok(ok) => awaiter.try_set_ok(ok.clone()),
                Err(err) => awaiter.try_set_error(err.clone()),
            };
        }

        result
    }

    /// Peeks the memorized result without executing anything. `None` means the key is
    /// unknown or is being executed right now.
    pub fn get_if_completed(&self, key: &str) -> Option<IdempotencyResult<TOk, TErr>> {
        let inner = self.inner.lock();

        let index = inner.find_index(key)?;

        match &inner.items[index].entry {
            IdempotencyEntry::Completed(result) => Some(result.clone()),
            IdempotencyEntry::Executing(_) => None,
        }
    }

    /// Amount of memorized results - never above `max_amount`.
    pub fn get_completed_amount(&self) -> usize {
        self.inner.lock().get_completed_amount()
    }

    /// Amount of executions which are in flight right now.
    pub fn get_executing_amount(&self) -> usize {
        let inner = self.inner.lock();
        inner.items.len() - inner.get_completed_amount()
    }
}

/// Owns the `Executing` entry for the duration of the execution.
///
/// [`ExecutionOwnerGuard::commit`] hands the entry over to the memorized result; if that
/// never happens (the owning future was cancelled, or the execution panicked and we are
/// unwinding), `Drop` removes the entry so the next retry can execute from scratch.
struct ExecutionOwnerGuard<'s, TOk, TErr> {
    inner: &'s Mutex<IdempotencyCacheInner<TOk, TErr>>,
    /// `None` once committed - that is what disarms `Drop`.
    key: Option<String>,
}

impl<'s, TOk, TErr> ExecutionOwnerGuard<'s, TOk, TErr> {
    fn new(inner: &'s Mutex<IdempotencyCacheInner<TOk, TErr>>, key: String) -> Self {
        Self {
            inner,
            key: Some(key),
        }
    }

    /// The key we are holding. Only valid before `commit` - which is the only place it is
    /// used from (diagnostics while the execution is still ours).
    fn get_key(&self) -> &str {
        match self.key.as_ref() {
            Some(key) => key.as_str(),
            None => "",
        }
    }

    /// Memorizes `result` and hands back everybody who parked while we were executing.
    fn commit(
        &mut self,
        result: IdempotencyResult<TOk, TErr>,
    ) -> Vec<TaskCompletion<Arc<TOk>, Arc<TErr>>> {
        let key = self
            .key
            .take()
            .expect("Idempotency execution is committed twice");

        let mut inner = self.inner.lock();

        // Unreachable while we hold the key: nobody else can remove our entry.
        let Some(index) = inner.find_index(key.as_str()) else {
            return Vec::new();
        };

        let Some(mut item) = inner.items.remove(index) else {
            return Vec::new();
        };

        let previous = std::mem::replace(&mut item.entry, IdempotencyEntry::Completed(result));

        // Back of the queue, so the eviction order stays "oldest completion first" even
        // when a slow execution finishes after ones which started later.
        inner.items.push_back(item);
        inner.gc();

        match previous {
            IdempotencyEntry::Executing(awaiters) => awaiters,
            IdempotencyEntry::Completed(_) => Vec::new(),
        }
    }
}

impl<'s, TOk, TErr> Drop for ExecutionOwnerGuard<'s, TOk, TErr> {
    fn drop(&mut self) {
        let Some(key) = self.key.take() else {
            return; // committed - nothing to clean up
        };

        let removed = {
            let mut inner = self.inner.lock();
            match inner.find_index(key.as_str()) {
                Some(index) => inner.items.remove(index),
                None => None,
            }
        };

        // Outside the lock: dropping the parked `TaskCompletion`s notifies their awaiters.
        drop(removed);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use tokio::sync::Semaphore;

    use super::*;

    fn create_runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    /// Lets other tasks reach their next await point.
    async fn yield_to_others() {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    enum TestOutcome {
        Ok,
        Err,
        Panic,
    }

    struct TestExecution {
        executions: Arc<AtomicUsize>,
        /// If set, an execution can only finish once the test grants it a permit.
        gate: Option<Arc<Semaphore>>,
        /// If set, only the execution with these params waits at the gate - the others
        /// run straight through.
        gated_param: Option<u64>,
        outcome: TestOutcome,
    }

    impl TestExecution {
        fn new(outcome: TestOutcome) -> Self {
            Self {
                executions: Arc::new(AtomicUsize::new(0)),
                gate: None,
                gated_param: None,
                outcome,
            }
        }

        fn gated(outcome: TestOutcome) -> (Self, Arc<Semaphore>) {
            let gate = Arc::new(Semaphore::new(0));
            (
                Self {
                    executions: Arc::new(AtomicUsize::new(0)),
                    gate: Some(gate.clone()),
                    gated_param: None,
                    outcome,
                },
                gate,
            )
        }

        fn gated_for_param(outcome: TestOutcome, param: u64) -> (Self, Arc<Semaphore>) {
            let (mut execution, gate) = Self::gated(outcome);
            execution.gated_param = Some(param);
            (execution, gate)
        }

        fn executions(&self) -> Arc<AtomicUsize> {
            self.executions.clone()
        }
    }

    #[async_trait::async_trait]
    impl IdempotencyExecution<u64, String, String> for TestExecution {
        async fn execute(&self, params: u64) -> Result<String, String> {
            self.executions.fetch_add(1, Ordering::SeqCst);

            let waits_at_the_gate = match self.gated_param {
                Some(gated_param) => params == gated_param,
                None => true,
            };

            if waits_at_the_gate {
                if let Some(gate) = self.gate.as_ref() {
                    gate.acquire().await.unwrap().forget();
                }
            }

            match self.outcome {
                TestOutcome::Ok => Ok(format!("ok:{}", params)),
                TestOutcome::Err => Err(format!("err:{}", params)),
                TestOutcome::Panic => panic!("execution panicked"),
            }
        }
    }

    type TestCache = IdempotencyCache<u64, String, String>;

    fn create_cache(
        execution: TestExecution,
        max_amount: usize,
    ) -> (Arc<TestCache>, Arc<AtomicUsize>) {
        // Deliberately the real default, so every test below runs against it.
        create_cache_with_timeout(execution, max_amount, DEFAULT_EXECUTION_TIMEOUT)
    }

    fn create_cache_with_timeout(
        execution: TestExecution,
        max_amount: usize,
        execution_timeout: Duration,
    ) -> (Arc<TestCache>, Arc<AtomicUsize>) {
        let executions = execution.executions();
        let cache: TestCache = IdempotencyCache::new_with_max_amount("test", max_amount)
            .set_execution_timeout(execution_timeout);

        let cache = Arc::new(cache);
        cache.register_execution(Arc::new(execution));
        (cache, executions)
    }

    #[test]
    fn retry_of_a_completed_key_does_not_execute_again() {
        create_runtime().block_on(async {
            let (cache, executions) = create_cache(TestExecution::new(TestOutcome::Ok), 10);

            let first = cache.execute("key".to_string(), 1).await;
            let retry = cache.execute("key".to_string(), 2).await;

            assert_eq!(first.as_ref().unwrap().as_str(), "ok:1");
            // The retry gets the memorized answer of the first call - its own params (2)
            // were never passed to the execution.
            assert_eq!(retry.as_ref().unwrap().as_str(), "ok:1");
            assert_eq!(executions.load(Ordering::SeqCst), 1);
            assert_eq!(cache.get_completed_amount(), 1);
        });
    }

    #[test]
    fn an_error_is_memorized_the_same_way_as_a_success() {
        create_runtime().block_on(async {
            let (cache, executions) = create_cache(TestExecution::new(TestOutcome::Err), 10);

            let first = cache.execute("key".to_string(), 1).await;
            let retry = cache.execute("key".to_string(), 1).await;

            assert_eq!(first.as_ref().unwrap_err().as_str(), "err:1");
            assert_eq!(retry.as_ref().unwrap_err().as_str(), "err:1");
            assert_eq!(executions.load(Ordering::SeqCst), 1);
        });
    }

    #[test]
    fn different_keys_are_executed_independently() {
        create_runtime().block_on(async {
            let (cache, executions) = create_cache(TestExecution::new(TestOutcome::Ok), 10);

            assert_eq!(
                cache.execute("a".to_string(), 1).await.unwrap().as_str(),
                "ok:1"
            );
            assert_eq!(
                cache.execute("b".to_string(), 2).await.unwrap().as_str(),
                "ok:2"
            );

            assert_eq!(executions.load(Ordering::SeqCst), 2);
            assert_eq!(cache.get_completed_amount(), 2);
        });
    }

    #[test]
    fn retries_arriving_during_the_execution_park_and_share_the_result() {
        create_runtime().block_on(async {
            let (execution, gate) = TestExecution::gated(TestOutcome::Ok);
            let (cache, executions) = create_cache(execution, 10);

            let owner = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute("key".to_string(), 1).await }
            });

            yield_to_others().await;

            let retries: Vec<_> = (0..3)
                .map(|_| {
                    tokio::spawn({
                        let cache = cache.clone();
                        async move { cache.execute("key".to_string(), 999).await }
                    })
                })
                .collect();

            yield_to_others().await;

            // Everybody is in flight on a single execution.
            assert_eq!(cache.get_executing_amount(), 1);
            assert_eq!(cache.get_completed_amount(), 0);
            assert!(cache.get_if_completed("key").is_none());

            gate.add_permits(1);

            assert_eq!(owner.await.unwrap().unwrap().as_str(), "ok:1");
            for retry in retries {
                assert_eq!(retry.await.unwrap().unwrap().as_str(), "ok:1");
            }

            assert_eq!(executions.load(Ordering::SeqCst), 1);
            assert_eq!(cache.get_completed_amount(), 1);
            assert_eq!(cache.get_executing_amount(), 0);
        });
    }

    #[test]
    fn parked_retries_get_the_error_too() {
        create_runtime().block_on(async {
            let (execution, gate) = TestExecution::gated(TestOutcome::Err);
            let (cache, executions) = create_cache(execution, 10);

            let owner = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute("key".to_string(), 1).await }
            });

            yield_to_others().await;

            let retry = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute("key".to_string(), 1).await }
            });

            yield_to_others().await;
            gate.add_permits(1);

            assert_eq!(owner.await.unwrap().unwrap_err().as_str(), "err:1");
            assert_eq!(retry.await.unwrap().unwrap_err().as_str(), "err:1");
            assert_eq!(executions.load(Ordering::SeqCst), 1);
        });
    }

    #[test]
    fn oldest_result_is_evicted_first() {
        create_runtime().block_on(async {
            let (cache, executions) = create_cache(TestExecution::new(TestOutcome::Ok), 2);

            cache.execute("a".to_string(), 1).await.unwrap();
            cache.execute("b".to_string(), 2).await.unwrap();
            // A cache hit must not refresh "a" - eviction is FIFO by completion time.
            cache.execute("a".to_string(), 1).await.unwrap();
            cache.execute("c".to_string(), 3).await.unwrap();

            assert_eq!(cache.get_completed_amount(), 2);
            // "a" was the oldest, so it is gone; "b" and "c" are still remembered.
            assert!(cache.get_if_completed("a").is_none());
            assert!(cache.get_if_completed("b").is_some());
            assert!(cache.get_if_completed("c").is_some());

            assert_eq!(executions.load(Ordering::SeqCst), 3);

            // "a" is forgotten, so it gets executed from scratch.
            cache.execute("a".to_string(), 1).await.unwrap();
            assert_eq!(executions.load(Ordering::SeqCst), 4);
        });
    }

    /// The queue holds `Executing` and `Completed` entries side by side, so eviction has
    /// to step over the in-flight ones instead of blindly dropping the front.
    #[test]
    fn in_flight_execution_is_not_evicted_by_newer_results() {
        create_runtime().block_on(async {
            // Only key "a" (params 1) parks at the gate; "b" and "c" run straight through.
            let (execution, gate) = TestExecution::gated_for_param(TestOutcome::Ok, 1);
            let (cache, executions) = create_cache(execution, 1);

            let owner = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute("a".to_string(), 1).await }
            });

            yield_to_others().await;
            assert_eq!(cache.get_executing_amount(), 1);

            // Two results complete while "a" is still in flight and sitting at the very
            // front of the queue. `max_amount` is 1, so gc runs on both of them.
            cache.execute("b".to_string(), 2).await.unwrap();
            cache.execute("c".to_string(), 3).await.unwrap();

            // "b" was evicted (it is the oldest *completed* one), "a" was not touched.
            assert_eq!(cache.get_completed_amount(), 1);
            assert!(cache.get_if_completed("b").is_none());
            assert!(cache.get_if_completed("c").is_some());
            assert_eq!(cache.get_executing_amount(), 1);

            // And "a" still completes normally, into its own awaiter.
            gate.add_permits(1);
            assert_eq!(owner.await.unwrap().unwrap().as_str(), "ok:1");
            assert_eq!(executions.load(Ordering::SeqCst), 3);
            assert_eq!(cache.get_executing_amount(), 0);
        });
    }

    #[test]
    fn zero_max_amount_still_de_duplicates_concurrent_retries() {
        create_runtime().block_on(async {
            let (execution, gate) = TestExecution::gated(TestOutcome::Ok);
            let (cache, executions) = create_cache(execution, 0);

            let owner = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute("key".to_string(), 1).await }
            });

            yield_to_others().await;

            let retry = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute("key".to_string(), 1).await }
            });

            yield_to_others().await;
            gate.add_permits(1);

            assert_eq!(owner.await.unwrap().unwrap().as_str(), "ok:1");
            assert_eq!(retry.await.unwrap().unwrap().as_str(), "ok:1");
            assert_eq!(executions.load(Ordering::SeqCst), 1);

            // Nothing is remembered afterwards.
            assert_eq!(cache.get_completed_amount(), 0);
            assert!(cache.get_if_completed("key").is_none());
        });
    }

    #[test]
    fn cancelled_owner_releases_the_key_and_panics_the_awaiters() {
        create_runtime().block_on(async {
            let (execution, gate) = TestExecution::gated(TestOutcome::Ok);
            let (cache, executions) = create_cache(execution, 10);

            let owner = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute("key".to_string(), 1).await }
            });

            yield_to_others().await;

            let retry = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute("key".to_string(), 1).await }
            });

            yield_to_others().await;
            assert_eq!(cache.get_executing_amount(), 1);

            owner.abort();
            yield_to_others().await;

            // The awaiter is released with a panic - we do not know whether the side
            // effect happened.
            assert!(retry.await.unwrap_err().is_panic());

            // Nothing is memorized and the key is free again.
            assert_eq!(cache.get_executing_amount(), 0);
            assert_eq!(cache.get_completed_amount(), 0);

            // So the next request starts from scratch.
            gate.add_permits(1);
            assert_eq!(
                cache.execute("key".to_string(), 2).await.unwrap().as_str(),
                "ok:2"
            );
            assert_eq!(executions.load(Ordering::SeqCst), 2);
        });
    }

    #[test]
    fn panicking_execution_releases_the_key() {
        create_runtime().block_on(async {
            let (cache, executions) = create_cache(TestExecution::new(TestOutcome::Panic), 10);

            let owner = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute("key".to_string(), 1).await }
            });

            assert!(owner.await.unwrap_err().is_panic());

            // Unwinding through the guard cleaned the entry up.
            assert_eq!(cache.get_executing_amount(), 0);
            assert_eq!(cache.get_completed_amount(), 0);
            assert_eq!(executions.load(Ordering::SeqCst), 1);

            // And the key can be executed again.
            let owner = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute("key".to_string(), 1).await }
            });
            assert!(owner.await.unwrap_err().is_panic());
            assert_eq!(executions.load(Ordering::SeqCst), 2);
        });
    }

    #[test]
    fn cancelled_retry_does_not_break_the_owner() {
        create_runtime().block_on(async {
            let (execution, gate) = TestExecution::gated(TestOutcome::Ok);
            let (cache, executions) = create_cache(execution, 10);

            let owner = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute("key".to_string(), 1).await }
            });

            yield_to_others().await;

            let retry = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute("key".to_string(), 1).await }
            });

            yield_to_others().await;

            // The retry goes away while parked - completing it must not panic the owner.
            retry.abort();
            yield_to_others().await;

            gate.add_permits(1);

            assert_eq!(owner.await.unwrap().unwrap().as_str(), "ok:1");
            assert_eq!(executions.load(Ordering::SeqCst), 1);
            assert_eq!(cache.get_completed_amount(), 1);
        });
    }

    /// The `Err` twin of the test above - it is the one that pins `try_set_error` rather
    /// than `try_set_ok`. Without it, swapping `try_set_error` for the panicking
    /// `set_error` goes unnoticed, and in production that panic would hit the *owner*:
    /// the caller which did the real work and whose result is already memorized.
    #[test]
    fn cancelled_retry_does_not_break_the_owner_when_the_execution_fails() {
        create_runtime().block_on(async {
            let (execution, gate) = TestExecution::gated(TestOutcome::Err);
            let (cache, executions) = create_cache(execution, 10);

            let owner = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute("key".to_string(), 1).await }
            });

            yield_to_others().await;

            let retry = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute("key".to_string(), 1).await }
            });

            yield_to_others().await;

            retry.abort();
            yield_to_others().await;

            gate.add_permits(1);

            // The owner must get its own error back, not a panic from notifying a dead
            // subscription.
            assert_eq!(owner.await.unwrap().unwrap_err().as_str(), "err:1");
            assert_eq!(executions.load(Ordering::SeqCst), 1);
            assert_eq!(cache.get_completed_amount(), 1);
        });
    }

    /// A hung execution must not pin its key forever. The timeout is the third way to not
    /// produce a result, so it behaves exactly like the panic and the cancellation above.
    #[test]
    fn timed_out_execution_releases_the_key() {
        create_runtime().block_on(async {
            // The gate is never granted a permit, so the execution hangs until the timeout.
            let (execution, _gate) = TestExecution::gated(TestOutcome::Ok);
            let (cache, executions) =
                create_cache_with_timeout(execution, 10, Duration::from_millis(100));

            let owner = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute("key".to_string(), 1).await }
            });

            yield_to_others().await;

            let retry = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute("key".to_string(), 1).await }
            });

            yield_to_others().await;
            assert_eq!(cache.get_executing_amount(), 1);

            // Both go down: the owner on the timeout panic, the parked retry because the
            // guard dropped its subscription while unwinding.
            let owner_error = owner.await.unwrap_err();
            assert!(owner_error.is_panic());
            assert!(retry.await.unwrap_err().is_panic());

            // Nothing memorized, key free again.
            assert_eq!(cache.get_executing_amount(), 0);
            assert_eq!(cache.get_completed_amount(), 0);

            // So a later retry executes from scratch instead of parking forever.
            let (execution, gate) = TestExecution::gated(TestOutcome::Ok);
            let (fresh_cache, fresh_executions) =
                create_cache_with_timeout(execution, 10, Duration::from_millis(100));
            gate.add_permits(1);
            assert_eq!(
                fresh_cache
                    .execute("key".to_string(), 2)
                    .await
                    .unwrap()
                    .as_str(),
                "ok:2"
            );
            assert_eq!(fresh_executions.load(Ordering::SeqCst), 1);
            assert_eq!(executions.load(Ordering::SeqCst), 1);
        });
    }

    /// The timeout must not fire on an execution that finishes in time.
    #[test]
    fn execution_within_the_timeout_is_untouched() {
        create_runtime().block_on(async {
            let (execution, gate) = TestExecution::gated(TestOutcome::Ok);
            let (cache, executions) =
                create_cache_with_timeout(execution, 10, Duration::from_secs(30));

            let owner = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute("key".to_string(), 1).await }
            });

            yield_to_others().await;
            gate.add_permits(1);

            assert_eq!(owner.await.unwrap().unwrap().as_str(), "ok:1");
            assert_eq!(executions.load(Ordering::SeqCst), 1);
            assert_eq!(cache.get_completed_amount(), 1);
        });
    }

    #[test]
    #[should_panic(expected = "Execution is not registered")]
    fn execute_without_registered_execution_panics() {
        create_runtime().block_on(async {
            let cache: TestCache = IdempotencyCache::new("test");
            let _ = cache.execute("key".to_string(), 1).await;
        });
    }

    #[test]
    #[should_panic(expected = "Execution is already registered")]
    fn second_registration_panics() {
        let cache: TestCache = IdempotencyCache::new("test");
        cache.register_execution(Arc::new(TestExecution::new(TestOutcome::Ok)));
        cache.register_execution(Arc::new(TestExecution::new(TestOutcome::Ok)));
    }
}
