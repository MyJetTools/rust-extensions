use std::collections::VecDeque;
use std::future::Future;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use parking_lot::Mutex;
use tokio::time::{error::Elapsed, Timeout};

use crate::{StrOrString, TaskCompletion};

use super::{IdempotencyCacheItem, IdempotencyEntry, IdempotencyResult};

/// How many completed results are kept by default.
pub const DEFAULT_MAX_AMOUNT: usize = 1000;

/// How long a single execution is allowed to take by default.
pub const DEFAULT_EXECUTION_TIMEOUT: Duration = Duration::from_secs(5);

/// A single flat queue: new keys are pushed to the back, the oldest results are dropped
/// from the front. There is deliberately no index next to it - one container means there
/// is no second structure that could drift out of sync with this one.
struct IdempotencyCacheInner<TKey, TOk, TErr> {
    items: VecDeque<IdempotencyCacheItem<TKey, TOk, TErr>>,
    max_amount: usize,
}

impl<TKey, TOk, TErr> IdempotencyCacheInner<TKey, TOk, TErr> {
    fn find_index(&self, matches: impl Fn(&TKey) -> bool) -> Option<usize> {
        self.items.iter().position(|item| matches(item.key.as_ref()))
    }

    /// Finds the entry of an execution owner by identity rather than by `PartialEq`: the
    /// owner holds the very `Arc` it pushed, and that cannot match anybody else's entry.
    fn find_owned_index(&self, key: &Arc<TKey>) -> Option<usize> {
        self.items.iter().position(|item| Arc::ptr_eq(&item.key, key))
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

/// Everything the idempotency caches have in common: the queue of keys, claiming a key by
/// its first caller, parking the retries, memorizing the result and evicting it.
///
/// A cache flavour only decides what its key is made of and how its execution is called -
/// see [`super::by_process_id`] and [`super::by_user_id_and_process_id`]. The key is only
/// ever compared for equality (lookups are a linear scan), never hashed, ordered or cloned.
pub(crate) struct IdempotencyCore<TKey, TExecution: ?Sized, TOk, TErr> {
    inner: Mutex<IdempotencyCacheInner<TKey, TOk, TErr>>,
    /// Written once, read on every `execute`. A `OnceLock` rather than a `Mutex` or an
    /// `ArcSwap`: reading it is a single atomic load which hands back a *reference*, so
    /// the hot path never touches the `Arc` refcount at all.
    execution: OnceLock<Arc<TExecution>>,
    execution_timeout: Duration,
    name: Arc<String>,
}

impl<TKey: PartialEq, TExecution: ?Sized, TOk, TErr> IdempotencyCore<TKey, TExecution, TOk, TErr> {
    pub fn new(name: impl Into<StrOrString<'static>>, max_amount: usize) -> Self {
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

    pub fn set_execution_timeout(&mut self, execution_timeout: Duration) {
        self.execution_timeout = execution_timeout;
    }

    /// One-shot: a second call panics.
    pub fn register_execution(&self, execution: Arc<TExecution>) {
        if self.execution.set(execution).is_err() {
            panic!(
                "Execution is already registered for the idempotency cache {}",
                self.name
            );
        }
    }

    /// Borrowed, not cloned - the caller only needs it for the duration of its own
    /// `&self`, so the hot path costs one atomic load and no refcount traffic.
    fn get_execution(&self) -> &TExecution {
        match self.execution.get() {
            Some(execution) => execution.as_ref(),
            None => panic!(
                "Execution is not registered for the idempotency cache {}",
                self.name
            ),
        }
    }

    /// Either hands back the answer `key` already has, or makes us the owner of its
    /// execution:
    ///
    /// - completed key - its memorized result, immediately;
    /// - key in flight - parks on a [`TaskCompletion`] until the owner is done and comes
    ///   back with the very same result;
    /// - unknown key - [`IdempotencyClaim::Owner`]: execute it, then
    ///   [`ExecutionOwnerGuard::complete`] it.
    pub async fn claim(&self, key: TKey) -> IdempotencyClaim<'_, TKey, TExecution, TOk, TErr> {
        // Resolved before we claim the key: panicking here after inserting the `Executing`
        // entry would leave that entry stuck in the queue forever.
        let execution = self.get_execution();

        let awaiter = {
            let mut inner = self.inner.lock();

            match inner.find_index(|item_key| *item_key == key) {
                Some(index) => match &mut inner.items[index].entry {
                    IdempotencyEntry::Completed(result) => {
                        return IdempotencyClaim::Answered(result.clone());
                    }
                    IdempotencyEntry::Executing(awaiters) => {
                        let mut task_completion = TaskCompletion::new();
                        let awaiter = task_completion.get_awaiter();
                        awaiters.push(task_completion);
                        awaiter
                    }
                },
                None => {
                    let key = Arc::new(key);

                    inner.items.push_back(IdempotencyCacheItem {
                        key: key.clone(),
                        entry: IdempotencyEntry::Executing(Vec::new()),
                    });

                    // From here on we own the execution of this key. The guard makes sure
                    // the `Executing` entry never outlives us - see `ExecutionOwnerGuard`.
                    return IdempotencyClaim::Owner(ExecutionOwnerGuard {
                        core: self,
                        execution,
                        key,
                        armed: true,
                    });
                }
            }
        };

        IdempotencyClaim::Answered(awaiter.get_result().await)
    }

    /// Peeks the memorized result without executing anything. `None` means the key is
    /// unknown or is being executed right now.
    pub fn get_if_completed(
        &self,
        matches: impl Fn(&TKey) -> bool,
    ) -> Option<IdempotencyResult<TOk, TErr>> {
        let inner = self.inner.lock();

        let index = inner.find_index(matches)?;

        match &inner.items[index].entry {
            IdempotencyEntry::Completed(result) => Some(result.clone()),
            IdempotencyEntry::Executing(_) => None,
        }
    }

    pub fn get_completed_amount(&self) -> usize {
        self.inner.lock().get_completed_amount()
    }

    pub fn get_executing_amount(&self) -> usize {
        let inner = self.inner.lock();
        inner.items.len() - inner.get_completed_amount()
    }
}

/// What [`IdempotencyCore::claim`] ends up with.
pub(crate) enum IdempotencyClaim<'s, TKey, TExecution: ?Sized, TOk, TErr> {
    /// The key already has an answer - a memorized one, or the one of the execution we
    /// parked on. There is nothing left to do but hand it back.
    Answered(IdempotencyResult<TOk, TErr>),
    /// The key is ours to execute.
    Owner(ExecutionOwnerGuard<'s, TKey, TExecution, TOk, TErr>),
}

/// Owns the `Executing` entry of a key for the duration of its execution.
///
/// [`ExecutionOwnerGuard::complete`] hands the entry over to the memorized result; if that
/// never happens (the owning future was cancelled, or the execution panicked or timed out
/// and we are unwinding), `Drop` removes the entry so the next retry can execute from
/// scratch, and drops the parked `TaskCompletion`s, which makes their awaiters panic with
/// "Task is dropped".
pub(crate) struct ExecutionOwnerGuard<'s, TKey, TExecution: ?Sized, TOk, TErr> {
    core: &'s IdempotencyCore<TKey, TExecution, TOk, TErr>,
    execution: &'s TExecution,
    /// The very `Arc` which sits in our queue entry - that is how we find the entry again.
    key: Arc<TKey>,
    /// Cleared by `complete` - that is what disarms `Drop`.
    armed: bool,
}

impl<'s, TKey, TExecution: ?Sized, TOk, TErr> ExecutionOwnerGuard<'s, TKey, TExecution, TOk, TErr> {
    pub fn get_execution(&self) -> &'s TExecution {
        self.execution
    }

    /// The key we are executing - to be handed to the execution itself.
    pub fn get_key(&self) -> &TKey {
        self.key.as_ref()
    }

    /// Caps `execution` at the execution timeout of the cache. The outcome goes to
    /// [`ExecutionOwnerGuard::complete`], overrun included.
    pub fn with_timeout<TFuture: Future>(&self, execution: TFuture) -> Timeout<TFuture> {
        tokio::time::timeout(self.core.execution_timeout, execution)
    }

    /// Memorizes the result and releases everybody who parked while we were executing.
    ///
    /// An overrun is the third way to not produce a result, so it is handled like the other
    /// two: we panic while still armed, and the unwinding goes through `Drop`, which frees
    /// the key and releases the awaiters.
    pub fn complete(
        mut self,
        executed: Result<Result<TOk, TErr>, Elapsed>,
    ) -> IdempotencyResult<TOk, TErr> {
        let Ok(executed) = executed else {
            panic!(
                "Idempotency execution in the cache '{}' timed out after {:?}",
                self.core.name, self.core.execution_timeout
            );
        };

        let result = match executed {
            Ok(ok) => Ok(Arc::new(ok)),
            Err(err) => Err(Arc::new(err)),
        };

        let awaiters = self.commit(result.clone());

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

    /// Memorizes `result` and hands back everybody who parked while we were executing.
    fn commit(
        &mut self,
        result: IdempotencyResult<TOk, TErr>,
    ) -> Vec<TaskCompletion<Arc<TOk>, Arc<TErr>>> {
        self.armed = false;

        let mut inner = self.core.inner.lock();

        // Unreachable while we hold the key: nobody else can remove our entry.
        let Some(index) = inner.find_owned_index(&self.key) else {
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

impl<'s, TKey, TExecution: ?Sized, TOk, TErr> Drop
    for ExecutionOwnerGuard<'s, TKey, TExecution, TOk, TErr>
{
    fn drop(&mut self) {
        if !self.armed {
            return; // completed - nothing to clean up
        }

        let removed = {
            let mut inner = self.core.inner.lock();
            match inner.find_owned_index(&self.key) {
                Some(index) => inner.items.remove(index),
                None => None,
            }
        };

        // Outside the lock: dropping the parked `TaskCompletion`s notifies their awaiters.
        drop(removed);
    }
}
