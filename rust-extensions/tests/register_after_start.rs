//! Registering after `start()`.
//!
//! The four queues used to take a handler on a running queue: their status went from
//! `Working` back to "handler registered", and a second `start()` then put a second
//! loop on the same queue. The timers took a tick after `start()` and never executed
//! it on schedule, because the loop works with the ticks registered by then. Both now
//! panic, the way a second `start()` does.
#![cfg(all(feature = "with-tokio", not(target_arch = "wasm32")))]

use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rust_extensions::{
    ExactTimerInterval, Logger, MyExactTimer, MyTimer, MyTimerTick, PersistObjectId,
    QueueToSave, QueueToSaveAsBulk, QueueToSaveAsBulkEventsHandler, QueueToSaveEventsHandler,
    QueueToSaveOrDeleteWithId, QueueToSaveOrDeleteWithIdEventsHandler, QueueToSaveWithId,
    QueueToSaveWithIdEventsHandler, RepeatTimerIteration, UpsertOrDelete,
};

struct TestLogger;

impl Logger for TestLogger {
    fn write_info(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
    fn write_warning(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
    fn write_error(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
    fn write_fatal_error(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
    fn write_debug_info(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
}

fn logger() -> Arc<dyn Logger + Send + Sync + 'static> {
    Arc::new(TestLogger)
}

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn panic_message(f: impl FnOnce()) -> String {
    let panic = std::panic::catch_unwind(AssertUnwindSafe(f)).expect_err("must panic");

    if let Some(message) = panic.downcast_ref::<String>() {
        return message.clone();
    }

    panic.downcast_ref::<&str>().unwrap().to_string()
}

#[derive(Clone, Debug)]
struct Item(u32);

impl PersistObjectId<u32> for Item {
    fn get_persist_object_id(&self) -> &u32 {
        &self.0
    }
}

/// Counts how many items are being handled at the same time, and the most it saw.
#[derive(Default)]
struct Concurrency {
    now: AtomicUsize,
    max: AtomicUsize,
    done: AtomicUsize,
}

struct SlowHandler {
    concurrency: Arc<Concurrency>,
}

#[async_trait::async_trait]
impl QueueToSaveEventsHandler<Item> for SlowHandler {
    async fn execute(&self, _: Item) {
        let now = self.concurrency.now.fetch_add(1, Ordering::SeqCst) + 1;
        self.concurrency.max.fetch_max(now, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(30)).await;
        self.concurrency.now.fetch_sub(1, Ordering::SeqCst);
        self.concurrency.done.fetch_add(1, Ordering::SeqCst);
    }
}

struct NamedHandler {
    name: &'static str,
    seen: Arc<Mutex<Vec<&'static str>>>,
}

#[async_trait::async_trait]
impl QueueToSaveEventsHandler<Item> for NamedHandler {
    async fn execute(&self, _: Item) {
        self.seen.lock().unwrap().push(self.name);
    }
}

struct NoopHandler;

#[async_trait::async_trait]
impl QueueToSaveAsBulkEventsHandler<Item> for NoopHandler {
    async fn execute(&self, _: &[Item], _: usize) {}
}

#[async_trait::async_trait]
impl QueueToSaveWithIdEventsHandler<Item> for NoopHandler {
    async fn execute(&self, _: &[Item], _: usize) {}
}

#[async_trait::async_trait]
impl QueueToSaveOrDeleteWithIdEventsHandler<u32, Item> for NoopHandler {
    async fn execute(&self, _: &[UpsertOrDelete<u32, Item>], _: usize) {}
}

struct NoopTick;

#[async_trait::async_trait]
impl MyTimerTick for NoopTick {
    async fn tick(&self) -> RepeatTimerIteration {
        RepeatTimerIteration::WithInterval
    }
}

#[test]
fn queue_to_save_refuses_a_handler_once_started_and_runs_one_loop() {
    rt().block_on(async {
        let concurrency = Arc::new(Concurrency::default());
        let handler = Arc::new(SlowHandler {
            concurrency: concurrency.clone(),
        });

        let queue = QueueToSave::new("q", logger());
        queue.register_events_handler(handler.clone());
        queue.start();

        assert_eq!(
            panic_message(|| queue.register_events_handler(handler.clone())),
            "QueueToSave q is already started"
        );
        // The status is still `Working`: a second start does not get through.
        assert_eq!(
            panic_message(|| queue.start()),
            "QueueToSave q is already started"
        );

        queue.enqueue((0..10).map(Item));
        while concurrency.done.load(Ordering::SeqCst) < 10 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // One loop hands the items over one at a time.
        assert_eq!(concurrency.max.load(Ordering::SeqCst), 1);
    });
}

#[test]
fn queue_to_save_takes_the_last_handler_registered_before_start() {
    rt().block_on(async {
        let seen = Arc::new(Mutex::new(Vec::new()));

        let queue = QueueToSave::new("q", logger());
        queue.register_events_handler(Arc::new(NamedHandler {
            name: "first",
            seen: seen.clone(),
        }));
        queue.register_events_handler(Arc::new(NamedHandler {
            name: "second",
            seen: seen.clone(),
        }));
        queue.start();

        queue.enqueue_single(Item(1));
        while seen.lock().unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        assert_eq!(*seen.lock().unwrap(), vec!["second"]);
    });
}

#[test]
fn queue_to_save_as_bulk_refuses_a_handler_once_started() {
    rt().block_on(async {
        let queue = QueueToSaveAsBulk::<Item>::new("bulk", logger());
        queue.register_events_handler(Arc::new(NoopHandler));
        queue.start();

        assert_eq!(
            panic_message(|| queue.register_events_handler(Arc::new(NoopHandler))),
            "QueueToSave bulk is already started"
        );
        assert_eq!(
            panic_message(|| queue.start()),
            "QueueToSave bulk is already started"
        );
    });
}

#[test]
fn queue_to_save_with_id_refuses_a_handler_once_started() {
    rt().block_on(async {
        let queue = QueueToSaveWithId::<u32, Item>::new("with-id", logger());
        queue.register_events_handler(Arc::new(NoopHandler));
        queue.start();

        assert_eq!(
            panic_message(|| queue.register_events_handler(Arc::new(NoopHandler))),
            "QueueToSaveWithId with-id is already started"
        );
        assert_eq!(
            panic_message(|| queue.start()),
            "QueueToSaveWithId with-id is already started"
        );
    });
}

#[test]
fn queue_to_save_or_delete_with_id_refuses_a_handler_once_started() {
    rt().block_on(async {
        let queue = QueueToSaveOrDeleteWithId::<u32, Item>::new("or-delete", logger());
        queue.register_events_handler(Arc::new(NoopHandler));
        queue.start();

        assert_eq!(
            panic_message(|| queue.register_events_handler(Arc::new(NoopHandler))),
            "QueueToSaveOrDeleteWithId or-delete is already started"
        );
        assert_eq!(
            panic_message(|| queue.start()),
            "QueueToSaveOrDeleteWithId or-delete is already started"
        );
    });
}

#[test]
fn my_timer_refuses_a_tick_once_started() {
    rt().block_on(async {
        let mut timer = MyTimer::new(Duration::from_secs(30), logger());
        timer.register_timer("early", Arc::new(NoopTick));
        timer.start();

        assert_eq!(
            panic_message(|| timer.register_timer("late", Arc::new(NoopTick))),
            "Timer [early] with interval 30s is already started: tick [late] must be registered before start()"
        );
    });
}

#[test]
fn my_exact_timer_refuses_a_tick_once_started() {
    rt().block_on(async {
        let mut timer = MyExactTimer::new(ExactTimerInterval::Every1Minute, logger());
        timer.register_timer("early", Arc::new(NoopTick));
        timer.start();

        let message = panic_message(|| timer.register_timer("late", Arc::new(NoopTick)));
        assert!(
            message.starts_with("Exact timer [early] with interval ")
                && message.ends_with(" is already started: tick [late] must be registered before start()"),
            "{message}"
        );
    });
}
