# idempotency

Feature `with-tokio`. Make a retried request execute at most once.

| Module | A request is identified by | Use when |
| --- | --- | --- |
| `idempotency::by_process_id` | a process id | the process id (typically the client's request id) is unique on its own |
| `idempotency::by_user_id_and_process_id` | user id + process id | a process id is unique only within its user |

For a given id:

- **The first call** runs the execution inline and remembers its `Result`, both `Ok` and `Err`.
- **A retry while the first call is still running** runs nothing. It waits and gets the very same result.
- **A retry after it finished** gets the remembered result at once.

Ids only need `PartialEq + Send + Sync + 'static` — they are never hashed, ordered or cloned.

## By process id

```rust
use std::{sync::Arc, time::Duration};
use rust_extensions::idempotency::by_process_id::{
    IdempotencyCache, IdempotencyExecution, IdempotencyResult, DEFAULT_MAX_AMOUNT,
};

pub struct ChargeParams {
    pub client_id: String,
    pub amount: f64,
}

struct ChargeExecution;

#[async_trait::async_trait]
impl IdempotencyExecution<String, ChargeParams, String, String> for ChargeExecution {
    async fn execute(&self, process_id: &String, params: ChargeParams) -> Result<String, String> {
        // the non-idempotent side effect - runs once per process id
        Ok(format!("{}: charged {} to {}", process_id, params.amount, params.client_id))
    }
}

pub struct AppCtx {
    pub charges: IdempotencyCache<String, ChargeParams, String, String>,
}

impl AppCtx {
    pub fn new() -> Self {
        Self {
            charges: IdempotencyCache::new_with_max_amount("charges", DEFAULT_MAX_AMOUNT) // or ::new
                .set_execution_timeout(Duration::from_secs(5)),
        }
    }
}

pub fn bootstrap(ctx: &AppCtx) {
    ctx.charges.register_execution(Arc::new(ChargeExecution));
}

pub async fn handle_request(ctx: &AppCtx, request_id: String, params: ChargeParams) {
    // Ok and Err come back behind an Arc - every retry shares the one result
    let result: IdempotencyResult<String, String> = ctx.charges.execute(request_id.clone(), params).await;

    match result {
        Ok(receipt) => println!("{}", receipt),
        Err(err) => println!("failed: {}", err),
    }

    let _remembered = ctx.charges.get_if_completed(&request_id); // peek, never executes
    let _sizes = (ctx.charges.get_completed_amount(), ctx.charges.get_executing_amount());
}
```

## By user id and process id

The same, with the user id in front. One user's retry gets the remembered answer, while another user with the very same process id is executed separately.

```rust
use std::sync::Arc;
use rust_extensions::idempotency::by_user_id_and_process_id::{IdempotencyCache, IdempotencyExecution};

pub struct Withdrawal {
    pub amount: f64,
}

struct WithdrawalExecution;

#[async_trait::async_trait]
impl IdempotencyExecution<i64, String, Withdrawal, String, String> for WithdrawalExecution {
    async fn execute(&self, user_id: &i64, process_id: &String, params: Withdrawal) -> Result<String, String> {
        Ok(format!("{}: user {} withdrew {}", process_id, user_id, params.amount))
    }
}

pub fn create() -> IdempotencyCache<i64, String, Withdrawal, String, String> {
    let cache = IdempotencyCache::new("withdrawals");
    cache.register_execution(Arc::new(WithdrawalExecution));
    cache
}

pub async fn withdraw(
    cache: &IdempotencyCache<i64, String, Withdrawal, String, String>,
    user_id: i64,
    process_id: String,
) {
    let _ = cache.execute(user_id, process_id, Withdrawal { amount: 10.0 }).await;
}
```

## Contracts

- **The last N results are kept.** N is `max_amount`, `DEFAULT_MAX_AMOUNT` = 1000. The oldest-completed result is evicted first, and a hit does not refresh an entry. `0` means "deduplicate concurrent retries, remember nothing". In `by_user_id_and_process_id` the cap is shared by all users. Lookups scan linearly — right for thousands, not for hundreds of thousands.
- **Executions are bounded.** Each runs under a timeout, `DEFAULT_EXECUTION_TIMEOUT` = 5s, changed with `set_execution_timeout`.
- **The first caller owns the execution.** If its future is dropped (an HTTP timeout), or the execution panics or times out, the entry is removed and nothing is remembered — nobody knows whether the side effect happened. The next retry executes from scratch. Retries already waiting get `get_result()` panicking with `"Task is dropped"`.
- **Register exactly once, before use.** A second `register_execution` panics, and `execute` before registration panics.
- **No lock is held across an `.await`.**
