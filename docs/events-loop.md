# events-loop

Feature `with-tokio`. `EventsLoop<TModel>` is a single-consumer message loop: many producers `send`, and one background task handles the events one by one, in order.

It lives inside an `AppCtx` as a plain field — every method takes `&self`, no outer `Mutex`.

```rust
use std::{sync::Arc, time::Duration};
use rust_extensions::{
    events_loop::{EventsLoop, EventsLoopTick, RepeatIteration},
    ApplicationStates, Logger,
};

pub enum Event {
    Deposit { account: u64, amount: f64 },
}

struct EventsHandler;

#[async_trait::async_trait]
impl EventsLoopTick<Event> for EventsHandler {
    async fn started(&self) {}

    async fn tick(&self, event: Event) -> RepeatIteration<Event> {
        match &event {
            Event::Deposit { account, amount } => {
                // ... handle it ...
                let _ = (account, amount);
            }
        }

        RepeatIteration::No // served - go for the next event
    }

    async fn finished(&self) {}
}

pub struct AppCtx {
    pub events: EventsLoop<Event>,
}

impl AppCtx {
    pub fn new(
        app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
        logger: Arc<dyn Logger + Send + Sync + 'static>,
    ) -> Self {
        Self {
            events: EventsLoop::new("events", app_states, logger)
                .set_iteration_timeout(Duration::from_secs(10)), // default 30s
        }
    }
}

pub fn bootstrap(ctx: &AppCtx) {
    ctx.events.register_event_loop(Arc::new(EventsHandler));
    ctx.events.start();
}

pub fn on_deposit(ctx: &AppCtx, account: u64, amount: f64) {
    ctx.events.send(Event::Deposit { account, amount }); // lock-free, never awaits
}
```

A producer that should not hold the whole loop takes a publisher — a cheap handle to the same channel:

```rust
use rust_extensions::events_loop::{EventsLoop, EventsLoopPublisher};

fn spawn_producer(events: &EventsLoop<String>) {
    let publisher: EventsLoopPublisher<String> = events.get_publisher();

    tokio::spawn(async move {
        publisher.send("from a background task".to_string());
        // publisher.stop() - the same Shutdown as EventsLoop::stop()
    });
}
```

## How it runs

- **The channel exists from `new`.** `send` works before `start()`. Events wait in the channel and are not lost.
- **It follows the application states.** The loop waits for `app_states.is_initialized()`, checking once a second, before it calls `started()` and takes the first event. `is_shutting_down()` is checked before each next event and before each repeat. Once it is raised the loop ends and calls `finished()`. A loop parked on an empty channel wakes on the next event, serves it, and only then sees the flag. `stop()` wakes it right away.
- **`stop()` sends `Shutdown` through the same channel.** Events queued before it are handled first.
- **After the loop ends, the receiver is gone.** `send` and `stop` panic from then on.
- **Register exactly once.** A second `register_event_loop` panics. `start()` without one, or a second `start()`, panics too.

## The event goes into the tick, and comes back to repeat

`tick` takes the event by value: nothing is cloned and `TModel` only has to be `Send`. An iteration that is not done with the event gives it back with `RepeatIteration::Yes(event)`. It is started again with that very event and a fresh timeout window, which is how a long job is done in portions.

```rust
use rust_extensions::events_loop::{EventsLoopTick, RepeatIteration};

pub struct Import {
    rows: Vec<String>,
}

struct ImportHandler;

#[async_trait::async_trait]
impl EventsLoopTick<Import> for ImportHandler {
    async fn started(&self) {}

    async fn tick(&self, mut import: Import) -> RepeatIteration<Import> {
        let portion: Vec<String> = import.rows.drain(..import.rows.len().min(1000)).collect();
        // save(portion).await;
        let _ = portion;

        if import.rows.is_empty() {
            RepeatIteration::No
        } else {
            RepeatIteration::Yes(import) // the rest, with a fresh timeout window
        }
    }

    async fn finished(&self) {}
}
```

**A panic or a timeout loses the event.** The event lived inside the failed future, so there is nothing to repeat with. Both are logged and the loop moves on. An event that has to survive a failing tick must be recovered by the tick itself: catch the error and answer `Yes(event)`.
