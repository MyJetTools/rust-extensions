# background-executor

Feature `with-tokio`. Move work off the caller's path. The caller signals "there may be work" with `trigger()` and returns at once, and a background task runs the registered job.

- `BackgroundExecutor` — one background task. Jobs never run in parallel.
- `BackgroundExecutorWithMultiThreads<TThreadId>` — one background task per thread id. The same id runs sequentially, different ids run in parallel.

The typical job is on-demand persistence. Callers change state and `trigger()`. The job takes whatever is pending and saves it, and returns quickly when nothing is pending, so an idle trigger is cheap.

## BackgroundExecutor

```rust
use std::sync::{Arc, Mutex};
use rust_extensions::{
    background_executor::{BackgroundExecutor, BackgroundJob, RepeatIteration},
    Logger,
};

struct FlushJob {
    pending: Arc<Mutex<Vec<u64>>>,
}

#[async_trait::async_trait]
impl BackgroundJob for FlushJob {
    async fn execute(&self) -> RepeatIteration {
        let batch: Vec<u64> = {
            let mut pending = self.pending.lock().unwrap();
            let take = pending.len().min(500);
            pending.drain(..take).collect()
        };

        // save(batch).await;

        if batch.len() == 500 {
            RepeatIteration::Yes // more left - run again, the trigger is not used up
        } else {
            RepeatIteration::No
        }
    }
}

pub struct AppCtx {
    pub pending: Arc<Mutex<Vec<u64>>>,
    pub flush: BackgroundExecutor,
}

impl AppCtx {
    pub fn new(logger: Arc<dyn Logger + Send + Sync + 'static>) -> Self {
        Self {
            pending: Arc::new(Mutex::new(Vec::new())),
            flush: BackgroundExecutor::new("flush", logger),
        }
    }
}

pub fn bootstrap(ctx: &AppCtx) {
    ctx.flush.register(Arc::new(FlushJob { pending: ctx.pending.clone() }));
    ctx.flush.start(); // from inside a Tokio runtime
}

pub fn on_change(ctx: &AppCtx, id: u64) {
    ctx.pending.lock().unwrap().push(id);
    ctx.flush.trigger(); // from any thread, even one without a runtime
}
```

- **One trigger, one `execute()`.** Triggers are counted, not merged. Triggers that arrive while a job runs are served afterwards.
- **`trigger()` is a semaphore permit.** It locks nothing, awaits nothing and spawns nothing, so it is legal from any thread, including an OS thread of a C++ host. Only `start()` needs a runtime.
- **Panics are caught and logged.** A panicked `execute()` uses up its trigger like `No`, so a job that always panics can not spin.
- **There is no timeout.** A hung `execute()` holds the executor.
- **Runs from `start()` on.** It does not wait for the application states, so start it once the application is ready.
- **Exactly one each.** A second `register()`, a second `start()`, a `start()` without `register()`, or a `trigger()` before `start()` panics.

## BackgroundExecutorWithMultiThreads

The same job contract, plus the thread id. Use it for per-entity work, such as flushing one account or one instrument. Entities must not block each other, but each one is saved in order.

```rust
use std::sync::Arc;
use rust_extensions::{
    background_executor_with_multi_threads::{
        BackgroundExecutorWithMultiThreads, BackgroundJobWithMultiThreads, RepeatIteration,
    },
    Logger,
};

struct FlushAccountJob;

#[async_trait::async_trait]
impl BackgroundJobWithMultiThreads<u64> for FlushAccountJob {
    async fn execute(&self, account_id: &u64) -> RepeatIteration {
        // save what is pending for this account only
        let _ = account_id;
        RepeatIteration::No
    }
}

pub fn create(logger: Arc<dyn Logger + Send + Sync + 'static>) -> BackgroundExecutorWithMultiThreads<u64> {
    let executor = BackgroundExecutorWithMultiThreads::new("flush-accounts", logger);
    executor.register(Arc::new(FlushAccountJob));
    executor.start(); // captures the runtime that readers are spawned on
    executor
}

pub fn on_account_changed(executor: &BackgroundExecutorWithMultiThreads<u64>, account_id: u64) {
    executor.trigger(account_id); // from any thread
}
```

- **Readers come and go.** The first trigger of a thread id spawns its reader, and the reader is removed once that id's triggers are drained. `get_working_threads_amount()` tells how many are alive.
- **Any id type works:** `Hash + Eq + Clone + Send + Sync + 'static` — `u64`, `String`, `Arc<String>`, a tuple.
- The panic, timeout and lifecycle rules are the same as for `BackgroundExecutor`.
