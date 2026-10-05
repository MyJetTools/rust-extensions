# timers

Feature `with-tokio`. Run code periodically.

- `MyTimer` sleeps `interval` between passes. A slow pass pushes the next one back.
- `MyExactTimer` fires on wall-clock marks: the marks are multiples of the interval counted from the Unix epoch, so `Every5Seconds` fires at `:00`, `:05`, `:10`… of every minute and `Every5Minutes` at `:00`, `:05`, … `:55` of every hour. It never drifts.

Both take the same `MyTimerTick`, so a tick moves from one timer to the other unchanged.

```rust
use std::{sync::Arc, time::Duration};
use rust_extensions::{
    ExactTimerInterval, Logger, MyExactTimer, MyTimer, MyTimerTick, RepeatTimerIteration,
};

struct FlushTick;

#[async_trait::async_trait]
impl MyTimerTick for FlushTick {
    async fn tick(&self) -> RepeatTimerIteration {
        // ... do the periodic work ...
        RepeatTimerIteration::WithInterval
    }
}

fn start_timers(logger: Arc<dyn Logger + Send + Sync + 'static>) -> (MyTimer, MyExactTimer) {
    let mut timer = MyTimer::new(Duration::from_secs(1), logger.clone());
    timer.set_first_tick_before_delay(); // the first pass right at start, not after one interval
    timer.set_iteration_timeout(Duration::from_secs(10)); // default 60s
    timer.register_timer("flush", Arc::new(FlushTick));
    timer.start();

    let mut exact = MyExactTimer::new(ExactTimerInterval::Every5Minutes, logger);
    exact.register_timer("report", Arc::new(FlushTick));
    exact.start();

    (timer, exact) // dropping them does not stop the loops
}
```

`ExactTimerInterval` values:

- `Every1Second`, `Every5Seconds`, `Every10Seconds`, `Every15Seconds`, `Every20Seconds`, `Every30Seconds`
- `Every1Minute`, `Every5Minutes`, `Every10Minutes`, `Every15Minutes`, `Every20Minutes`, `Every30Minutes`

`MyTimer::new_with_execute_timeout` and `MyExactTimer::new_with_execute_timeout` set the timeout at construction.

## Lifecycle

- **Register before `start()`.** Every tick is registered before `start()`; `register_timer` afterwards panics. A duplicate tick name panics too.
- **One `start()`.** A second `start()` panics instead of putting a second loop on the same ticks.
- **Running from `start()` on.** Neither timer waits for the application to be initialized, so start them once the application is ready.
- **Ticks of one timer run in parallel within a pass.** The next pass waits for all of them.
- **Timeouts.** Every tick of a pass gets `iteration_timeout` from the start of the pass. A tick that overruns it is cancelled and logged through the `Logger`.
- **Panics.** A panic is caught and logged, and the timer keeps going. A tick that panicked or timed out is not repeated.

## RepeatTimerIteration — a long job in portions

A tick with more work than fits into one timeout window returns `Immediately`. It is started again right away with a fresh timeout window, instead of racing the timeout.

```rust
use std::sync::Mutex;
use rust_extensions::{MyTimerTick, RepeatTimerIteration};

struct DrainTick {
    pending: Mutex<Vec<u64>>,
}

#[async_trait::async_trait]
impl MyTimerTick for DrainTick {
    async fn tick(&self) -> RepeatTimerIteration {
        let batch: Vec<u64> = {
            let mut pending = self.pending.lock().unwrap();
            let take = pending.len().min(100);
            pending.drain(..take).collect()
        };

        if batch.is_empty() {
            return RepeatTimerIteration::WithInterval;
        }

        // save(batch).await;

        RepeatTimerIteration::Immediately // more may be waiting
    }
}
```

- Only the tick that asked is repeated; its neighbours keep their schedule.
- The schedule does not shift. `MyExactTimer` computes the next mark after the extra passes, and `MyTimer` sleeps once they are done.
- A tick that always answers `Immediately` never lets the timer sleep.

## Running a tick by hand

`execute_timer(name)` runs a registered tick once, out of schedule, and hands its `RepeatTimerIteration` back instead of acting on it:

```rust
use rust_extensions::{MyTimer, RepeatTimerIteration};

async fn flush_now(timer: &MyTimer) {
    let answer: RepeatTimerIteration = timer.execute_timer("flush").await; // panics on an unknown name
    assert!(answer.is_with_interval() || answer.is_immediately());
}
```
