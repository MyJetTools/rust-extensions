use std::sync::Arc;
use std::time::Duration;

use crate::idempotency::{IdempotencyClaim, IdempotencyCore};
use crate::StrOrString;

use super::{IdempotencyExecution, IdempotencyResult, DEFAULT_MAX_AMOUNT};

type RegisteredExecution<TProcessId, TParams, TOk, TErr> =
    Arc<dyn IdempotencyExecution<TProcessId, TParams, TOk, TErr>>;

type Core<TProcessId, TParams, TOk, TErr> = IdempotencyCore<
    TProcessId,
    dyn IdempotencyExecution<TProcessId, TParams, TOk, TErr>,
    TOk,
    TErr,
>;

/// De-duplicates retries of the same request.
///
/// A request is identified by its process id (typically the client's request id). The
/// process id is any `TProcessId` which can be compared for equality - `PartialEq` is all
/// it takes, it is never hashed, ordered or cloned. For a given process id:
///
/// - **first call** - runs [`IdempotencyExecution::execute`] inline (right inside
///   `execute`) and memorizes its `Result`;
/// - **a retry while the first call is still running** - does not execute anything: it
///   parks on a [`TaskCompletion`](crate::TaskCompletion) and is released with the very
///   same result;
/// - **a retry after it finished** - gets the memorized result immediately, the
///   execution is not touched.
///
/// Both `Ok` and `Err` are memorized: once a process id produced an answer, every retry of
/// that process id gets that answer back.
///
/// The last `max_amount` results are kept, evicted **FIFO by completion time** (a cache
/// hit does not refresh the entry). `max_amount == 0` is legal and means "de-duplicate
/// concurrent retries, but do not remember anything afterwards". In-flight executions are
/// never eviction candidates, so they do not count against `max_amount`.
///
/// Lookups are a linear scan over a single queue, which is the right shape at these sizes
/// (comparing typical ids is cheap - a `String` comparison rejects on length first) and
/// keeps the whole state in one container. It is not the right shape for a `max_amount`
/// in the hundreds of thousands.
///
/// It is designed to live inside an `AppCtx` as a plain field - every method takes
/// `&self`, no outer `Mutex` needed.
///
/// When the same process id is only unique within a user, key the requests by both -
/// [`by_user_id_and_process_id::IdempotencyCache`](crate::idempotency::by_user_id_and_process_id::IdempotencyCache).
///
/// # Cancellation, timeouts and panics
///
/// The first caller owns the execution, so it also owns its fate. If that caller's future
/// is dropped (HTTP timeout, cancelled task), or the execution panics, or it runs longer
/// than the execution timeout, the entry is removed, so the next retry starts the execution
/// from scratch, and everybody parked on it gets the standard `TaskCompletion` drop
/// behaviour: their `get_result()` panics with `"Task is dropped"`. Nothing is memorized in
/// any of those cases, because we do not know whether the side effect happened.
///
/// The timeout is just the third way to not produce a result, so it is handled as a panic
/// like the other two: the execution future is dropped and the owner panics too. It also
/// bounds how long an `Executing` entry can hold its process id - without it a hung
/// execution would pin that process id forever and every retry of it would park forever.
/// Default [`DEFAULT_EXECUTION_TIMEOUT`](super::DEFAULT_EXECUTION_TIMEOUT), changed with
/// [`IdempotencyCache::set_execution_timeout`]. It needs a Tokio runtime with time enabled.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use rust_extensions::idempotency::by_process_id::{
///     IdempotencyCache, IdempotencyExecution, DEFAULT_MAX_AMOUNT,
/// };
///
/// pub struct ChargeParams {
///     pub amount: f64,
/// }
///
/// struct ChargeExecution;
///
/// #[async_trait::async_trait]
/// impl IdempotencyExecution<String, ChargeParams, String, String> for ChargeExecution {
///     async fn execute(&self, process_id: &String, params: ChargeParams) -> Result<String, String> {
///         // the real, non-idempotent work happens here exactly once per process id
///         Ok(format!("{}: charged {}", process_id, params.amount))
///     }
/// }
///
/// pub struct AppCtx {
///     pub charges: IdempotencyCache<String, ChargeParams, String, String>,
/// }
///
/// # async fn example() {
/// let ctx = AppCtx {
///     charges: IdempotencyCache::new_with_max_amount("charges", DEFAULT_MAX_AMOUNT),
/// };
/// ctx.charges.register_execution(Arc::new(ChargeExecution));
///
/// // Retrying this with the same process id never charges twice.
/// let result = ctx
///     .charges
///     .execute("request-id-1".to_string(), ChargeParams { amount: 10.0 })
///     .await;
/// # }
/// ```
pub struct IdempotencyCache<
    TProcessId: PartialEq + Send + Sync + 'static,
    TParams: Send + Sync + 'static,
    TOk: Send + Sync + 'static,
    TErr: Send + Sync + 'static,
> {
    core: Core<TProcessId, TParams, TOk, TErr>,
}

impl<
        TProcessId: PartialEq + Send + Sync + 'static,
        TParams: Send + Sync + 'static,
        TOk: Send + Sync + 'static,
        TErr: Send + Sync + 'static,
    > IdempotencyCache<TProcessId, TParams, TOk, TErr>
{
    /// Creates a cache which keeps the last [`DEFAULT_MAX_AMOUNT`] results.
    pub fn new(name: impl Into<StrOrString<'static>>) -> Self {
        Self::new_with_max_amount(name, DEFAULT_MAX_AMOUNT)
    }

    /// Creates a cache which keeps the last `max_amount` results.
    pub fn new_with_max_amount(name: impl Into<StrOrString<'static>>, max_amount: usize) -> Self {
        Self {
            core: IdempotencyCore::new(name, max_amount),
        }
    }

    /// Caps how long a single execution may take. Overrunning it is treated exactly like
    /// a panic - see the type documentation. Default
    /// [`DEFAULT_EXECUTION_TIMEOUT`](super::DEFAULT_EXECUTION_TIMEOUT).
    ///
    /// Builder style: `IdempotencyCache::new("charges").set_execution_timeout(timeout)`.
    pub fn set_execution_timeout(mut self, execution_timeout: Duration) -> Self {
        self.core.set_execution_timeout(execution_timeout);
        self
    }

    /// Registers the execution. One-shot: a second call panics.
    ///
    /// It is a separate step (not a constructor argument) so the execution is free to
    /// hold an `Arc` of the very `AppCtx` which owns this cache.
    pub fn register_execution(
        &self,
        execution: RegisteredExecution<TProcessId, TParams, TOk, TErr>,
    ) {
        self.core.register_execution(execution);
    }

    /// Returns the result of `process_id`, executing it only if it has to be executed.
    ///
    /// See the type documentation for what happens on a retry, on cancellation and on a
    /// panic. `params` is consumed only by the caller which actually executes; the ones
    /// which get a memorized result simply drop it.
    pub async fn execute(
        &self,
        process_id: TProcessId,
        params: TParams,
    ) -> IdempotencyResult<TOk, TErr> {
        let owner = match self.core.claim(process_id).await {
            IdempotencyClaim::Answered(result) => return result,
            IdempotencyClaim::Owner(owner) => owner,
        };

        // The execution is ours now. If this future is cancelled, or the execution panics
        // or overruns the timeout, dropping `owner` frees the process id.
        let execution = owner.get_execution().execute(owner.get_key(), params);
        let executed = owner.with_timeout(execution).await;
        owner.complete(executed)
    }

    /// Peeks the memorized result without executing anything. `None` means the process id
    /// is unknown or is being executed right now.
    pub fn get_if_completed(
        &self,
        process_id: &TProcessId,
    ) -> Option<IdempotencyResult<TOk, TErr>> {
        self.core
            .get_if_completed(|item_process_id| item_process_id == process_id)
    }

    /// Amount of memorized results - never above `max_amount`.
    pub fn get_completed_amount(&self) -> usize {
        self.core.get_completed_amount()
    }

    /// Amount of executions which are in flight right now.
    pub fn get_executing_amount(&self) -> usize {
        self.core.get_executing_amount()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use tokio::sync::Semaphore;

    use super::super::DEFAULT_EXECUTION_TIMEOUT;
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
    impl IdempotencyExecution<String, u64, String, String> for TestExecution {
        async fn execute(&self, _process_id: &String, params: u64) -> Result<String, String> {
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

    type TestCache = IdempotencyCache<String, u64, String, String>;

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
            assert!(cache.get_if_completed(&"key".to_string()).is_none());

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
            assert!(cache.get_if_completed(&"a".to_string()).is_none());
            assert!(cache.get_if_completed(&"b".to_string()).is_some());
            assert!(cache.get_if_completed(&"c".to_string()).is_some());

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
            assert!(cache.get_if_completed(&"b".to_string()).is_none());
            assert!(cache.get_if_completed(&"c".to_string()).is_some());
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
            assert!(cache.get_if_completed(&"key".to_string()).is_none());
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

    /// Implements nothing but `PartialEq` - no `Clone`, `Hash` or `Debug`. That this
    /// compiles is what pins "comparable for equality" as the whole contract of a process id.
    #[derive(PartialEq)]
    struct ProcessId(u64);

    struct ProcessIdEcho;

    #[async_trait::async_trait]
    impl IdempotencyExecution<ProcessId, u64, String, String> for ProcessIdEcho {
        async fn execute(&self, process_id: &ProcessId, params: u64) -> Result<String, String> {
            Ok(format!("{}:{}", process_id.0, params))
        }
    }

    #[test]
    fn process_id_needs_nothing_but_partial_eq_and_reaches_the_execution() {
        create_runtime().block_on(async {
            let cache: IdempotencyCache<ProcessId, u64, String, String> =
                IdempotencyCache::new("test");
            cache.register_execution(Arc::new(ProcessIdEcho));

            assert_eq!(cache.execute(ProcessId(7), 1).await.unwrap().as_str(), "7:1");
            // The retry is recognized by `PartialEq` alone and gets the memorized answer.
            assert_eq!(cache.execute(ProcessId(7), 2).await.unwrap().as_str(), "7:1");
            assert_eq!(cache.execute(ProcessId(8), 3).await.unwrap().as_str(), "8:3");

            assert!(cache.get_if_completed(&ProcessId(7)).is_some());
            assert!(cache.get_if_completed(&ProcessId(9)).is_none());
            assert_eq!(cache.get_completed_amount(), 2);
        });
    }
}
