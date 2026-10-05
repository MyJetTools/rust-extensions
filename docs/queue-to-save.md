# queue-to-save

Feature `with-tokio`. Write-behind queues: producers `enqueue` and return, and one background loop hands the items to your handler, which saves them to a database, a file or a remote service.

| Queue | The handler gets | If the handler fails |
| --- | --- | --- |
| `QueueToSave<T>` | one item, by value | lost (logged) |
| `QueueToSaveAsBulk<T>` | up to 50 items, `&[T]` | the same chunk is retried |
| `QueueToSaveWithId<ID, T>` | up to 50 items, the latest state per ID | the same chunk is retried |
| `QueueToSaveOrDeleteWithId<ID, T>` | up to 50 upserts or deletes, the latest per ID | the same chunk is retried |

"Fails" means the handler panicked or ran longer than 10 seconds. A retried chunk is handed over again after `retry_timeout`, 1s by default, set with `set_retry_timeout`, and `attempt_no` grows by one each time. There is no retry limit: the same chunk comes back until the handler returns, and returning means "saved". A handler that should give up has to decide that itself, from `attempt_no`.

## QueueToSave

```rust
use std::sync::Arc;
use rust_extensions::{Logger, QueueToSave, QueueToSaveEventsHandler};

pub struct AuditRecord {
    pub text: String,
}

struct AuditWriter;

#[async_trait::async_trait]
impl QueueToSaveEventsHandler<AuditRecord> for AuditWriter {
    async fn execute(&self, record: AuditRecord) {
        // write(record).await; - a panic or a timeout loses this record
        let _ = record.text;
    }
}

pub fn create(logger: Arc<dyn Logger + Send + Sync + 'static>) -> QueueToSave<AuditRecord> {
    let queue = QueueToSave::new("audit", logger);
    queue.register_events_handler(Arc::new(AuditWriter));
    queue.start();
    queue
}

pub fn audit(queue: &QueueToSave<AuditRecord>, text: &str) {
    queue.enqueue_single(AuditRecord { text: text.to_string() });
    let _waiting = queue.queue_len(); // the item being handled is not counted
}
```

## QueueToSaveAsBulk

```rust
use std::{sync::Arc, time::Duration};
use rust_extensions::{Logger, QueueToSaveAsBulk, QueueToSaveAsBulkEventsHandler};

struct RowsWriter;

#[async_trait::async_trait]
impl QueueToSaveAsBulkEventsHandler<String> for RowsWriter {
    async fn execute(&self, rows: &[String], attempt_no: usize) {
        // bulk_insert(rows).await - panic on failure to get the same rows again
        let _ = (rows, attempt_no);
    }
}

pub fn create(logger: Arc<dyn Logger + Send + Sync + 'static>) -> QueueToSaveAsBulk<String> {
    let queue = QueueToSaveAsBulk::new("rows", logger).set_retry_timeout(Duration::from_secs(5));
    queue.register_events_handler(Arc::new(RowsWriter));
    queue.start();

    queue.enqueue(["a".to_string(), "b".to_string()].into_iter());
    queue
}
```

## QueueToSaveWithId — the latest state per ID

Enqueuing an item whose ID is already pending replaces the pending item, so a record changed ten times between flushes is saved once, as it is now. There is no ordering guarantee across IDs. `ID: Hash + Eq + Clone + Debug`.

```rust
use std::sync::Arc;
use rust_extensions::{Logger, PersistObjectId, QueueToSaveWithId, QueueToSaveWithIdEventsHandler};

pub struct Balance {
    pub account: u64,
    pub value: f64,
}

impl PersistObjectId<u64> for Balance {
    fn get_persist_object_id(&self) -> &u64 {
        &self.account
    }
}

struct BalancesWriter;

#[async_trait::async_trait]
impl QueueToSaveWithIdEventsHandler<Balance> for BalancesWriter {
    async fn execute(&self, balances: &[Balance], attempt_no: usize) {
        // upsert(balances).await
        let _ = (balances, attempt_no);
    }
}

pub fn create(logger: Arc<dyn Logger + Send + Sync + 'static>) -> QueueToSaveWithId<u64, Balance> {
    let queue = QueueToSaveWithId::new("balances", logger);
    queue.register_events_handler(Arc::new(BalancesWriter));
    queue.start();

    queue.enqueue_single(Balance { account: 1, value: 10.0 });
    queue.enqueue_single(Balance { account: 1, value: 15.0 }); // replaces the pending 10.0
    queue
}
```

A panic prints the IDs of the chunk to the console before the retry.

## QueueToSaveOrDeleteWithId — upserts and deletes

Each ID is pending as an upsert or as a delete.

- `enqueue_delete(id)` drops a pending upsert of that ID right away, since there is nothing to save about an object that is about to be deleted. Only the ID stays, marked for deletion.
- A later `enqueue_single` of the same ID turns the delete back into an upsert.

`UpsertOrDelete::split` cuts a chunk into one bulk upsert and one bulk delete.

```rust
use std::sync::Arc;
use rust_extensions::{
    Logger, PersistObjectId, QueueToSaveOrDeleteWithId, QueueToSaveOrDeleteWithIdEventsHandler,
    UpsertOrDelete,
};

pub struct Session {
    pub id: String,
    pub user: u64,
}

impl PersistObjectId<String> for Session {
    fn get_persist_object_id(&self) -> &String {
        &self.id
    }
}

struct SessionsWriter;

#[async_trait::async_trait]
impl QueueToSaveOrDeleteWithIdEventsHandler<String, Session> for SessionsWriter {
    async fn execute(&self, items: &[UpsertOrDelete<String, Session>], attempt_no: usize) {
        let (to_upsert, to_delete) = UpsertOrDelete::split(items);
        // upsert(to_upsert).await; delete(to_delete).await;
        let _ = (to_upsert, to_delete, attempt_no);

        for item in items {
            let _id: &String = item.get_id();
            if let Some(session) = item.as_upsert() {
                let _ = session.user;
            }
        }
    }
}

pub fn create(
    logger: Arc<dyn Logger + Send + Sync + 'static>,
) -> QueueToSaveOrDeleteWithId<String, Session> {
    let queue = QueueToSaveOrDeleteWithId::new("sessions", logger);
    queue.register_events_handler(Arc::new(SessionsWriter));
    queue.start();

    queue.enqueue_single(Session { id: "s1".into(), user: 1 });
    queue.enqueue_delete("s1".to_string()); // the pending upsert is dropped
    queue.enqueue_delete_multiple(["s2".to_string(), "s3".to_string()].into_iter());
    queue
}
```

## Lifecycle — the same for all four

- `register_events_handler` comes before `start()`. Calling it again before `start()` replaces the handler. After `start()` it panics.
- A second `start()`, or a `start()` without a handler, panics.
- The loop runs from `start()` on, without waiting for the application states.
- Every queue implements [`Startable`](app-lifecycle.md).
