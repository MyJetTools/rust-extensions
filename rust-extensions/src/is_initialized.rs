use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::sync::Mutex;

use crate::TaskCompletion;

/// A one-shot initialization gate.
///
/// Any amount of processes can `await` [`IsInitialized::wait_until_initialized`] and
/// they all stay parked until initialization happens. Once [`IsInitialized::initialized`]
/// is called every parked awaiter is released, and every subsequent
/// `wait_until_initialized` call returns immediately (it just flies through the
/// atomic flag without touching the mutex).
pub struct IsInitialized {
    initialized: AtomicBool,
    awaiters: Mutex<Vec<TaskCompletion<(), ()>>>,
}

impl IsInitialized {
    pub fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            awaiters: Mutex::new(Vec::new()),
        }
    }

    /// Returns whether initialization has already happened.
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Relaxed)
    }

    /// Panics with `"Not Initialized"` if initialization has not happened yet.
    pub fn panic_if_not_initialized(&self) {
        if !self.is_initialized() {
            panic!("Not Initialized");
        }
    }

    /// Awaits initialization for at most `duration`. Returns as soon as it happens;
    /// panics with `"Not Initialized"` if it does not happen within `duration`.
    pub async fn wait_some_time_and_panic(&self, duration: Duration) {
        if tokio::time::timeout(duration, self.wait_until_initialized())
            .await
            .is_err()
        {
            panic!("Not Initialized");
        }
    }

    /// Awaits until [`IsInitialized::initialized`] is called.
    ///
    /// If initialization has already happened the call returns immediately via the
    /// atomic flag; otherwise the caller subscribes (a `TaskCompletion` is stored)
    /// and parks until initialization releases it.
    pub async fn wait_until_initialized(&self) {
        // Fast path: already initialized -> fly through without locking.
        if self.initialized.load(Ordering::Acquire) {
            return;
        }

        let awaiter = {
            let mut awaiters = self.awaiters.lock().await;

            // Re-check under the lock: `initialized` sets the flag and drains the
            // vec while holding the same mutex, so either we observe the flag here
            // and return, or we push before the drain and get completed by it.
            if self.initialized.load(Ordering::Relaxed) {
                return;
            }

            let mut task_completion = TaskCompletion::new();
            // If this gate is dropped un-initialized, release waiters with an error
            // instead of the default panic-on-drop.
            task_completion.set_drop_error(());
            let awaiter = task_completion.get_awaiter();
            awaiters.push(task_completion);
            awaiter
        };

        let _ = awaiter.get_result().await;
    }

    /// Marks the gate as initialized: completes every parked awaiter and raises the
    /// atomic flag so all further `wait_until_initialized` calls fly through.
    ///
    /// Idempotent - calling it again is a no-op (the vec is already drained).
    pub async fn initialized(&self) {
        let mut awaiters = self.awaiters.lock().await;

        self.initialized.store(true, Ordering::Release);

        for mut task_completion in awaiters.drain(..) {
            // `try_set_ok` (not `set_ok`) because an awaiter may have been cancelled
            // (its receiver dropped), which would make `set_ok` panic.
            let _ = task_completion.try_set_ok(());
        }
    }
}

impl Default for IsInitialized {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use super::*;

    fn create_runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    #[test]
    fn returns_immediately_when_already_initialized() {
        create_runtime().block_on(async {
            let is_initialized = IsInitialized::new();
            is_initialized.initialized().await;

            assert!(is_initialized.is_initialized());
            // Must fly through without hanging.
            is_initialized.wait_until_initialized().await;
        });
    }

    #[test]
    fn parked_waiters_are_released_by_initialized() {
        create_runtime().block_on(async {
            let is_initialized = Arc::new(IsInitialized::new());

            let waiter_1 = tokio::spawn({
                let is_initialized = is_initialized.clone();
                async move { is_initialized.wait_until_initialized().await }
            });
            let waiter_2 = tokio::spawn({
                let is_initialized = is_initialized.clone();
                async move { is_initialized.wait_until_initialized().await }
            });

            // Let both waiters register their subscriptions and park.
            tokio::time::sleep(Duration::from_millis(50)).await;
            assert!(!is_initialized.is_initialized());

            is_initialized.initialized().await;
            assert!(is_initialized.is_initialized());

            // Both must complete now.
            waiter_1.await.unwrap();
            waiter_2.await.unwrap();
        });
    }

    #[test]
    fn initialized_is_idempotent() {
        create_runtime().block_on(async {
            let is_initialized = IsInitialized::new();
            is_initialized.initialized().await;
            is_initialized.initialized().await;
            is_initialized.wait_until_initialized().await;
        });
    }

    #[test]
    fn panic_if_not_initialized_does_not_panic_after_init() {
        create_runtime().block_on(async {
            let is_initialized = IsInitialized::new();
            is_initialized.initialized().await;
            is_initialized.panic_if_not_initialized();
        });
    }

    #[test]
    #[should_panic(expected = "Not Initialized")]
    fn panic_if_not_initialized_panics_before_init() {
        let is_initialized = IsInitialized::new();
        is_initialized.panic_if_not_initialized();
    }

    #[test]
    #[should_panic(expected = "Not Initialized")]
    fn wait_some_time_and_panic_panics_on_timeout() {
        create_runtime().block_on(async {
            let is_initialized = IsInitialized::new();
            is_initialized
                .wait_some_time_and_panic(Duration::from_millis(50))
                .await;
        });
    }

    #[test]
    fn wait_some_time_and_panic_returns_when_initialized_in_time() {
        create_runtime().block_on(async {
            let is_initialized = Arc::new(IsInitialized::new());

            let waiter = tokio::spawn({
                let is_initialized = is_initialized.clone();
                async move {
                    is_initialized
                        .wait_some_time_and_panic(Duration::from_secs(30))
                        .await;
                }
            });

            // Initialize well before the timeout elapses.
            tokio::time::sleep(Duration::from_millis(50)).await;
            is_initialized.initialized().await;

            waiter.await.unwrap();
        });
    }

    #[test]
    fn cancelled_waiter_does_not_break_initialized() {
        create_runtime().block_on(async {
            let is_initialized = Arc::new(IsInitialized::new());

            let waiter = tokio::spawn({
                let is_initialized = is_initialized.clone();
                async move { is_initialized.wait_until_initialized().await }
            });

            // Let it subscribe and park, then cancel it (drops its receiver).
            tokio::time::sleep(Duration::from_millis(50)).await;
            waiter.abort();
            tokio::time::sleep(Duration::from_millis(50)).await;

            // Completing the (now dead) subscription must not panic.
            is_initialized.initialized().await;
            assert!(is_initialized.is_initialized());
        });
    }
}
