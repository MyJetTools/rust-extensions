# app-lifecycle

Feature `with-tokio`. The traits that wire an application together: the logger the background components report to, the application states they read, and one `start()` for all of them.

## Logger

Every background component (timers, events loop, executors, queues) takes an `Arc<dyn Logger + Send + Sync>` and reports panics and timeouts through it. The application implements it once:

```rust
use std::collections::HashMap;
use rust_extensions::Logger;

pub struct ConsoleLogger;

impl Logger for ConsoleLogger {
    fn write_info(&self, process: String, message: String, _ctx: Option<HashMap<String, String>>) {
        println!("INFO  [{}] {}", process, message);
    }

    fn write_warning(&self, process: String, message: String, _ctx: Option<HashMap<String, String>>) {
        println!("WARN  [{}] {}", process, message);
    }

    fn write_error(&self, process: String, message: String, _ctx: Option<HashMap<String, String>>) {
        eprintln!("ERROR [{}] {}", process, message);
    }

    fn write_fatal_error(&self, process: String, message: String, _ctx: Option<HashMap<String, String>>) {
        eprintln!("FATAL [{}] {}", process, message);
    }

    fn write_debug_info(&self, process: String, message: String, _ctx: Option<HashMap<String, String>>) {
        println!("DEBUG [{}] {}", process, message);
    }
}
```

## ApplicationStates / AppStates

`ApplicationStates` is how a component reads the lifecycle: `is_initialized()` and `is_shutting_down()`. Of the components here only [`EventsLoop`](events-loop.md) reads it.

`AppStates` is the ready implementation: two atomic flags, plus `wait_until_shutdown()`, which hooks SIGTERM / SIGINT. It is not available on wasm.

```rust
use rust_extensions::{AppStates, ApplicationStates};

let states = AppStates::create_un_initialized(); // or create_initialized()
assert!(!states.is_initialized());

states.set_initialized();
assert!(states.is_initialized());

states.set_shutting_down();
assert!(states.is_shutting_down());

// In main: blocks until SIGTERM / SIGINT raises the shutdown flag.
async fn run_until_signal(states: &AppStates) {
    states.wait_until_shutdown().await;
}
```

## Startable — start everything in one loop

`fn start(&self)` is implemented by:

- `MyTimer`, `MyExactTimer`
- `EventsLoop`
- `BackgroundExecutor`, `BackgroundExecutorWithMultiThreads`
- `QueueToSave`, `QueueToSaveAsBulk`, `QueueToSaveWithId`, `QueueToSaveOrDeleteWithId`

`new` gets everything `start` needs, so the components can be collected while the application is wired up and started together. Each type also has `start()` as an inherent method, so starting a single one needs no import.

```rust
use std::{sync::Arc, time::Duration};
use rust_extensions::{
    background_executor::{BackgroundExecutor, BackgroundJob, RepeatIteration},
    AppStates, Logger, MyTimer, MyTimerTick, RepeatTimerIteration, Startable,
};

struct Tick;

#[async_trait::async_trait]
impl MyTimerTick for Tick {
    async fn tick(&self) -> RepeatTimerIteration {
        RepeatTimerIteration::WithInterval
    }
}

struct Job;

#[async_trait::async_trait]
impl BackgroundJob for Job {
    async fn execute(&self) -> RepeatIteration {
        RepeatIteration::No
    }
}

fn wire_up(logger: Arc<dyn Logger + Send + Sync + 'static>) -> Vec<Arc<dyn Startable + Send + Sync>> {
    let mut timer = MyTimer::new(Duration::from_secs(1), logger.clone());
    timer.register_timer("tick", Arc::new(Tick));

    let executor = BackgroundExecutor::new("job", logger);
    executor.register(Arc::new(Job));

    vec![Arc::new(timer), Arc::new(executor)]
}

// Inside the Tokio runtime, once the application is ready:
fn start_all(to_start: &[Arc<dyn Startable + Send + Sync>], app_states: &AppStates) {
    for component in to_start {
        component.start(); // each one exactly once - a second start() panics
    }

    app_states.set_initialized();
}
```

Only `EventsLoop` waits for `is_initialized()`. Timers, executors and queues work from `start()` on, so start them once the application is ready for them.
