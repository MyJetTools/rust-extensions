use std::sync::Arc;

use crate::TaskCompletion;

/// What a retry of an already known key gets back.
///
/// Both sides are `Arc` - the result is handed out to every retry of the same key, so
/// sharing it costs one atomic increment and neither `TOk` nor `TErr` has to be `Clone`.
pub type IdempotencyResult<TOk, TErr> = Result<Arc<TOk>, Arc<TErr>>;

pub(crate) enum IdempotencyEntry<TOk, TErr> {
    /// Somebody is executing this key right now. Everybody else who came in meanwhile
    /// parked here - each of them owns the awaiter of one of these `TaskCompletion`s.
    Executing(Vec<TaskCompletion<Arc<TOk>, Arc<TErr>>>),
    /// The execution is over and its result is memorized. Both `Ok` and `Err` land here:
    /// a retry of this key never re-executes anything.
    Completed(IdempotencyResult<TOk, TErr>),
}

impl<TOk, TErr> IdempotencyEntry<TOk, TErr> {
    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Completed(_))
    }
}

pub(crate) struct IdempotencyCacheItem<TOk, TErr> {
    pub key: String,
    pub entry: IdempotencyEntry<TOk, TErr>,
}
