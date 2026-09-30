use std::sync::Arc;
use std::time::Duration;

use crate::idempotency::{IdempotencyClaim, IdempotencyCore};
use crate::StrOrString;

use super::{IdempotencyExecution, IdempotencyResult, DEFAULT_MAX_AMOUNT};

type RegisteredExecution<TUserId, TProcessId, TParams, TOk, TErr> =
    Arc<dyn IdempotencyExecution<TUserId, TProcessId, TParams, TOk, TErr>>;

type Core<TUserId, TProcessId, TParams, TOk, TErr> = IdempotencyCore<
    UserIdAndProcessId<TUserId, TProcessId>,
    dyn IdempotencyExecution<TUserId, TProcessId, TParams, TOk, TErr>,
    TOk,
    TErr,
>;

/// The key of a request - a process id is only unique within its user.
#[derive(PartialEq)]
struct UserIdAndProcessId<TUserId, TProcessId> {
    user_id: TUserId,
    process_id: TProcessId,
}

/// De-duplicates retries of the same request of a user.
///
/// The same as
/// [`by_process_id::IdempotencyCache`](crate::idempotency::by_process_id::IdempotencyCache),
/// except that a request is identified by a **pair**: the user it is made on behalf of, and
/// its process id. So a process id only has to be unique within its user - the same process
/// id of two different users is two independent requests. Both ids are any types which can
/// be compared for equality - `PartialEq` is all it takes, they are never hashed, ordered or
/// cloned. For a given pair:
///
/// - **first call** - runs [`IdempotencyExecution::execute`] inline, handing it both ids,
///   and memorizes its `Result`;
/// - **a retry while the first call is still running** - does not execute anything: it
///   parks and is released with the very same result;
/// - **a retry after it finished** - gets the memorized result immediately.
///
/// Everything else - memorized errors, FIFO eviction, `max_amount == 0`, cancellation,
/// timeouts and panics - works exactly as described on
/// [`by_process_id::IdempotencyCache`](crate::idempotency::by_process_id::IdempotencyCache).
///
/// `max_amount` caps the memorized answers of the whole cache, not of each user: the
/// eviction is FIFO by completion time across all users.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use rust_extensions::idempotency::by_user_id_and_process_id::{
///     IdempotencyCache, IdempotencyExecution, DEFAULT_MAX_AMOUNT,
/// };
///
/// pub struct WithdrawalParams {
///     pub amount: f64,
/// }
///
/// struct WithdrawalExecution;
///
/// #[async_trait::async_trait]
/// impl IdempotencyExecution<i64, String, WithdrawalParams, String, String>
///     for WithdrawalExecution
/// {
///     async fn execute(
///         &self,
///         user_id: &i64,
///         process_id: &String,
///         params: WithdrawalParams,
///     ) -> Result<String, String> {
///         // the real, non-idempotent work happens here exactly once per user and process id
///         Ok(format!("{}: user {} withdrew {}", process_id, user_id, params.amount))
///     }
/// }
///
/// pub struct AppCtx {
///     pub withdrawals: IdempotencyCache<i64, String, WithdrawalParams, String, String>,
/// }
///
/// # async fn example() {
/// let ctx = AppCtx {
///     withdrawals: IdempotencyCache::new_with_max_amount("withdrawals", DEFAULT_MAX_AMOUNT),
/// };
/// ctx.withdrawals.register_execution(Arc::new(WithdrawalExecution));
///
/// // Retrying this never withdraws twice - while another user is free to use the very
/// // same process id.
/// let result = ctx
///     .withdrawals
///     .execute(42, "withdrawal-1".to_string(), WithdrawalParams { amount: 10.0 })
///     .await;
/// # }
/// ```
pub struct IdempotencyCache<
    TUserId: PartialEq + Send + Sync + 'static,
    TProcessId: PartialEq + Send + Sync + 'static,
    TParams: Send + Sync + 'static,
    TOk: Send + Sync + 'static,
    TErr: Send + Sync + 'static,
> {
    core: Core<TUserId, TProcessId, TParams, TOk, TErr>,
}

impl<
        TUserId: PartialEq + Send + Sync + 'static,
        TProcessId: PartialEq + Send + Sync + 'static,
        TParams: Send + Sync + 'static,
        TOk: Send + Sync + 'static,
        TErr: Send + Sync + 'static,
    > IdempotencyCache<TUserId, TProcessId, TParams, TOk, TErr>
{
    /// Creates a cache which keeps the last [`DEFAULT_MAX_AMOUNT`] results.
    pub fn new(name: impl Into<StrOrString<'static>>) -> Self {
        Self::new_with_max_amount(name, DEFAULT_MAX_AMOUNT)
    }

    /// Creates a cache which keeps the last `max_amount` results - of all users together.
    pub fn new_with_max_amount(name: impl Into<StrOrString<'static>>, max_amount: usize) -> Self {
        Self {
            core: IdempotencyCore::new(name, max_amount),
        }
    }

    /// Caps how long a single execution may take. Overrunning it is treated exactly like
    /// a panic. Default [`DEFAULT_EXECUTION_TIMEOUT`](super::DEFAULT_EXECUTION_TIMEOUT).
    ///
    /// Builder style: `IdempotencyCache::new("withdrawals").set_execution_timeout(timeout)`.
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
        execution: RegisteredExecution<TUserId, TProcessId, TParams, TOk, TErr>,
    ) {
        self.core.register_execution(execution);
    }

    /// Returns the result of `process_id` of `user_id`, executing it only if it has to be
    /// executed.
    ///
    /// `params` is consumed only by the caller which actually executes; the ones which get
    /// a memorized result simply drop it.
    pub async fn execute(
        &self,
        user_id: TUserId,
        process_id: TProcessId,
        params: TParams,
    ) -> IdempotencyResult<TOk, TErr> {
        let key = UserIdAndProcessId {
            user_id,
            process_id,
        };

        let owner = match self.core.claim(key).await {
            IdempotencyClaim::Answered(result) => return result,
            IdempotencyClaim::Owner(owner) => owner,
        };

        // The execution is ours now. If this future is cancelled, or the execution panics
        // or overruns the timeout, dropping `owner` frees the pair of ids.
        let key = owner.get_key();
        let execution = owner
            .get_execution()
            .execute(&key.user_id, &key.process_id, params);
        let executed = owner.with_timeout(execution).await;
        owner.complete(executed)
    }

    /// Peeks the memorized result without executing anything. `None` means the pair of ids
    /// is unknown or is being executed right now.
    pub fn get_if_completed(
        &self,
        user_id: &TUserId,
        process_id: &TProcessId,
    ) -> Option<IdempotencyResult<TOk, TErr>> {
        self.core
            .get_if_completed(|key| key.user_id == *user_id && key.process_id == *process_id)
    }

    /// Amount of memorized results of all users - never above `max_amount`.
    pub fn get_completed_amount(&self) -> usize {
        self.core.get_completed_amount()
    }

    /// Amount of executions which are in flight right now.
    pub fn get_executing_amount(&self) -> usize {
        self.core.get_executing_amount()
    }
}

/// The retry / eviction / cancellation / timeout machinery is shared with
/// `by_process_id::IdempotencyCache` and is exercised in depth by its tests. These pin what
/// is specific to this flavour: a request is the pair of ids, and both reach the execution.
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

    /// The ids implement nothing but `PartialEq` - no `Clone`, `Hash` or `Debug`. That this
    /// compiles is what pins "comparable for equality" as the whole contract of the ids.
    #[derive(PartialEq)]
    struct UserId(&'static str);

    #[derive(PartialEq)]
    struct ProcessId(u64);

    struct TestExecution {
        executions: Arc<AtomicUsize>,
        /// If set, an execution can only finish once the test grants it a permit.
        gate: Option<Arc<Semaphore>>,
    }

    #[async_trait::async_trait]
    impl IdempotencyExecution<UserId, ProcessId, u64, String, String> for TestExecution {
        async fn execute(
            &self,
            user_id: &UserId,
            process_id: &ProcessId,
            params: u64,
        ) -> Result<String, String> {
            self.executions.fetch_add(1, Ordering::SeqCst);

            if let Some(gate) = self.gate.as_ref() {
                gate.acquire().await.unwrap().forget();
            }

            Ok(format!("{}:{}:{}", user_id.0, process_id.0, params))
        }
    }

    type TestCache = IdempotencyCache<UserId, ProcessId, u64, String, String>;

    fn create_cache(
        gate: Option<Arc<Semaphore>>,
        max_amount: usize,
    ) -> (Arc<TestCache>, Arc<AtomicUsize>) {
        let executions = Arc::new(AtomicUsize::new(0));

        let cache: TestCache = IdempotencyCache::new_with_max_amount("test", max_amount);
        cache.register_execution(Arc::new(TestExecution {
            executions: executions.clone(),
            gate,
        }));

        (Arc::new(cache), executions)
    }

    #[test]
    fn retry_of_the_same_user_and_process_id_does_not_execute_again() {
        create_runtime().block_on(async {
            let (cache, executions) = create_cache(None, 10);

            let first = cache.execute(UserId("alice"), ProcessId(1), 100).await;
            let retry = cache.execute(UserId("alice"), ProcessId(1), 200).await;

            // The execution got both ids. The retry gets the memorized answer - its own
            // params (200) were never passed to the execution.
            assert_eq!(first.unwrap().as_str(), "alice:1:100");
            assert_eq!(retry.unwrap().as_str(), "alice:1:100");
            assert_eq!(executions.load(Ordering::SeqCst), 1);
            assert_eq!(cache.get_completed_amount(), 1);
        });
    }

    #[test]
    fn the_same_process_id_of_different_users_is_executed_independently() {
        create_runtime().block_on(async {
            let (cache, executions) = create_cache(None, 10);

            let alice = cache.execute(UserId("alice"), ProcessId(1), 100).await;
            let bob = cache.execute(UserId("bob"), ProcessId(1), 200).await;

            // Bob does not get the answer memorized for alice's process id 1.
            assert_eq!(alice.unwrap().as_str(), "alice:1:100");
            assert_eq!(bob.unwrap().as_str(), "bob:1:200");
            assert_eq!(executions.load(Ordering::SeqCst), 2);
            assert_eq!(cache.get_completed_amount(), 2);
        });
    }

    #[test]
    fn different_process_ids_of_the_same_user_are_executed_independently() {
        create_runtime().block_on(async {
            let (cache, executions) = create_cache(None, 10);

            let first = cache.execute(UserId("alice"), ProcessId(1), 100).await;
            let second = cache.execute(UserId("alice"), ProcessId(2), 200).await;

            assert_eq!(first.unwrap().as_str(), "alice:1:100");
            assert_eq!(second.unwrap().as_str(), "alice:2:200");
            assert_eq!(executions.load(Ordering::SeqCst), 2);
        });
    }

    #[test]
    fn get_if_completed_needs_both_ids_to_match() {
        create_runtime().block_on(async {
            let (cache, _) = create_cache(None, 10);

            cache
                .execute(UserId("alice"), ProcessId(1), 100)
                .await
                .unwrap();

            let completed = cache.get_if_completed(&UserId("alice"), &ProcessId(1));
            assert_eq!(completed.unwrap().unwrap().as_str(), "alice:1:100");

            assert!(cache
                .get_if_completed(&UserId("bob"), &ProcessId(1))
                .is_none());
            assert!(cache
                .get_if_completed(&UserId("alice"), &ProcessId(2))
                .is_none());
        });
    }

    #[test]
    fn retries_arriving_during_the_execution_park_and_share_the_result() {
        create_runtime().block_on(async {
            let gate = Arc::new(Semaphore::new(0));
            let (cache, executions) = create_cache(Some(gate.clone()), 10);

            let owner = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute(UserId("alice"), ProcessId(1), 100).await }
            });

            yield_to_others().await;

            let retry = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute(UserId("alice"), ProcessId(1), 999).await }
            });

            yield_to_others().await;

            // One execution in flight, the retry is parked on it.
            assert_eq!(cache.get_executing_amount(), 1);
            assert!(cache
                .get_if_completed(&UserId("alice"), &ProcessId(1))
                .is_none());

            gate.add_permits(1);

            assert_eq!(owner.await.unwrap().unwrap().as_str(), "alice:1:100");
            assert_eq!(retry.await.unwrap().unwrap().as_str(), "alice:1:100");
            assert_eq!(executions.load(Ordering::SeqCst), 1);
            assert_eq!(cache.get_executing_amount(), 0);
        });
    }

    /// The in-flight twin of the test above: it goes through the `Executing` branch of the
    /// lookup rather than the `Completed` one.
    #[test]
    fn another_user_does_not_park_on_an_in_flight_process_id() {
        create_runtime().block_on(async {
            let gate = Arc::new(Semaphore::new(0));
            let (cache, executions) = create_cache(Some(gate.clone()), 10);

            let alice = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute(UserId("alice"), ProcessId(1), 100).await }
            });

            yield_to_others().await;

            let bob = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute(UserId("bob"), ProcessId(1), 200).await }
            });

            yield_to_others().await;

            // Bob's request is not a retry of alice's - both are executing.
            assert_eq!(cache.get_executing_amount(), 2);
            assert_eq!(executions.load(Ordering::SeqCst), 2);

            gate.add_permits(2);

            assert_eq!(alice.await.unwrap().unwrap().as_str(), "alice:1:100");
            assert_eq!(bob.await.unwrap().unwrap().as_str(), "bob:1:200");
            assert_eq!(cache.get_completed_amount(), 2);
        });
    }

    #[test]
    fn cancelled_owner_releases_the_pair_of_ids() {
        create_runtime().block_on(async {
            let gate = Arc::new(Semaphore::new(0));
            let (cache, executions) = create_cache(Some(gate.clone()), 10);

            let owner = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute(UserId("alice"), ProcessId(1), 100).await }
            });

            yield_to_others().await;

            let retry = tokio::spawn({
                let cache = cache.clone();
                async move { cache.execute(UserId("alice"), ProcessId(1), 100).await }
            });

            yield_to_others().await;

            owner.abort();
            yield_to_others().await;

            // We do not know whether the side effect happened: the parked retry is
            // released with a panic and nothing is memorized.
            assert!(retry.await.unwrap_err().is_panic());
            assert_eq!(cache.get_executing_amount(), 0);
            assert_eq!(cache.get_completed_amount(), 0);

            // The pair is free again, so the next request executes from scratch.
            gate.add_permits(1);
            let result = cache.execute(UserId("alice"), ProcessId(1), 200).await;
            assert_eq!(result.unwrap().as_str(), "alice:1:200");
            assert_eq!(executions.load(Ordering::SeqCst), 2);
        });
    }

    /// `max_amount` caps the whole cache, not each user.
    #[test]
    fn eviction_is_fifo_across_all_users() {
        create_runtime().block_on(async {
            let (cache, _) = create_cache(None, 2);

            cache
                .execute(UserId("alice"), ProcessId(1), 100)
                .await
                .unwrap();
            cache
                .execute(UserId("bob"), ProcessId(1), 200)
                .await
                .unwrap();
            cache
                .execute(UserId("alice"), ProcessId(2), 300)
                .await
                .unwrap();

            // The oldest answer of the whole cache is gone, whichever user it belonged to.
            assert_eq!(cache.get_completed_amount(), 2);
            assert!(cache
                .get_if_completed(&UserId("alice"), &ProcessId(1))
                .is_none());
            assert!(cache
                .get_if_completed(&UserId("bob"), &ProcessId(1))
                .is_some());
            assert!(cache
                .get_if_completed(&UserId("alice"), &ProcessId(2))
                .is_some());
        });
    }
}
