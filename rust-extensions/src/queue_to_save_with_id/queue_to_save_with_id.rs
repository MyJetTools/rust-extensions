use std::{fmt::Debug, hash::Hash, panic::AssertUnwindSafe, sync::Arc, time::Duration};

use futures::FutureExt;
use parking_lot::Mutex;

use crate::{Logger, StrOrString};

use super::{inner_with_id::QueueToSaveInnerWithId, persist_object_id::PersistObjectId};

enum HandlerStatus<T> {
    None,
    Some(Arc<dyn QueueToSaveWithIdEventsHandler<T> + Send + Sync + 'static>),
    Working,
}

pub struct QueueToSaveWithId<ID, T>
where
    ID: Hash + Eq + Clone + Debug + Send + Sync + 'static,
    T: PersistObjectId<ID> + Send + Sync + 'static,
{
    inner: Arc<QueueToSaveInnerWithId<ID, T>>,
    handler: Mutex<HandlerStatus<T>>,
    retry_timeout: Duration,
}

impl<ID, T> QueueToSaveWithId<ID, T>
where
    ID: Hash + Eq + Clone + Debug + Send + Sync + 'static,
    T: PersistObjectId<ID> + Send + Sync + 'static,
{
    pub fn new(name: impl Into<StrOrString<'static>>) -> Self {
        Self {
            inner: Arc::new(QueueToSaveInnerWithId::new(name.into())),
            handler: Mutex::new(HandlerStatus::None),
            retry_timeout: Duration::from_secs(1),
        }
    }

    /// The pause before a chunk which was not saved is handed over again.
    pub fn set_retry_timeout(mut self, retry_timeout: Duration) -> Self {
        self.retry_timeout = retry_timeout;
        self
    }

    pub fn enqueue(&self, items: impl Iterator<Item = T>) {
        self.inner.enqueue(items);
    }

    pub fn enqueue_single(&self, item: T) {
        self.inner.enqueue_single(item);
    }

    pub fn register_events_handler(
        &self,
        events_handle: Arc<dyn QueueToSaveWithIdEventsHandler<T> + Send + Sync + 'static>,
    ) {
        let mut write_access = self.handler.lock();
        *write_access = HandlerStatus::Some(events_handle);
    }

    pub fn get_name(&self) -> &str {
        self.inner.name.as_str()
    }

    pub fn start(&self, logger: Arc<dyn Logger + Send + Sync + 'static>) {
        let mut write_access = self.handler.lock();

        match &*write_access {
            HandlerStatus::None => {
                panic!(
                    "Event handler is not registered in QueueToSaveWithId {}",
                    self.inner.name
                );
            }
            HandlerStatus::Some(handler) => {
                tokio::spawn(queue_to_save_with_id_loop(
                    self.inner.clone(),
                    handler.clone(),
                    logger,
                    self.retry_timeout,
                ));
            }
            HandlerStatus::Working => {
                panic!(
                    "QueueToSaveWithId {} is already started",
                    self.inner.name
                );
            }
        }

        *write_access = HandlerStatus::Working;
    }
}

#[async_trait::async_trait]
pub trait QueueToSaveWithIdEventsHandler<T: Send + Sync + 'static> {
    /// Returning means the items are saved. A panic or a timeout means they are
    /// not - the very same items are handed over again after `retry_timeout`,
    /// until it returns.
    ///
    /// `attempt_no` is 0 on the first attempt and grows by one on every retry of
    /// the same items.
    async fn execute(&self, items: &[T], attempt_no: usize);
}

async fn queue_to_save_with_id_loop<ID, T>(
    inner: Arc<QueueToSaveInnerWithId<ID, T>>,
    handler: Arc<dyn QueueToSaveWithIdEventsHandler<T> + Send + Sync + 'static>,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
    retry_timeout: Duration,
) where
    ID: Hash + Eq + Clone + Debug + Send + Sync + 'static,
    T: PersistObjectId<ID> + Send + Sync + 'static,
{
    println!("QueueToSaveWithId {} is started", inner.name.as_str());
    let timeout = inner.timeout;
    loop {
        let events = inner.dequeue().await;
        let mut attempt_no = 0;

        // The chunk is handed over by reference until the handler returns - a
        // chunk it panicked on is not lost, it is saved again.
        loop {
            let future = AssertUnwindSafe(handler.execute(&events, attempt_no)).catch_unwind();

            let msg = match tokio::time::timeout(timeout, future).await {
                Ok(Ok(())) => break,
                Ok(Err(_)) => {
                    let ids: Vec<&ID> = events
                        .iter()
                        .map(|itm| itm.get_persist_object_id())
                        .collect();

                    println!(
                        "QueueToSaveWithId {}: handler panicked on the items with IDs: {:?}",
                        inner.name.as_str(),
                        ids
                    );

                    format!(
                        "Panic at QueueToSaveWithIdEventsHandler named {}. {} item(s) are going to be saved again",
                        inner.name.as_str(),
                        events.len()
                    )
                }
                Err(_) => format!(
                    "Timeout {:?} at QueueToSaveWithIdEventsHandler named {}. {} item(s) are going to be saved again",
                    timeout,
                    inner.name.as_str(),
                    events.len()
                ),
            };

            logger.write_error("QueueToSaveWithId.loop".to_string(), msg, None.into());
            attempt_no += 1;

            tokio::time::sleep(retry_timeout).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::Mutex;

    use crate::Logger;

    use super::*;

    #[derive(Clone, Debug)]
    struct Obj {
        id: u32,
        value: &'static str,
    }

    impl PersistObjectId<u32> for Obj {
        fn get_persist_object_id(&self) -> &u32 {
            &self.id
        }
    }

    struct CapturingHandler {
        captured: Arc<Mutex<Vec<Obj>>>,
        notify: Arc<tokio::sync::Notify>,
    }

    #[async_trait::async_trait]
    impl QueueToSaveWithIdEventsHandler<Obj> for CapturingHandler {
        async fn execute(&self, items: &[Obj], _attempt_no: usize) {
            let mut guard = self.captured.lock().await;
            guard.extend_from_slice(items);
            self.notify.notify_one();
        }
    }

    struct NoopLogger;

    impl Logger for NoopLogger {
        fn write_info(
            &self,
            _: String,
            _: String,
            _: Option<std::collections::HashMap<String, String>>,
        ) {
        }
        fn write_warning(
            &self,
            _: String,
            _: String,
            _: Option<std::collections::HashMap<String, String>>,
        ) {
        }
        fn write_error(
            &self,
            _: String,
            _: String,
            _: Option<std::collections::HashMap<String, String>>,
        ) {
        }
        fn write_fatal_error(
            &self,
            _: String,
            _: String,
            _: Option<std::collections::HashMap<String, String>>,
        ) {
        }
        fn write_debug_info(
            &self,
            _: String,
            _: String,
            _: Option<std::collections::HashMap<String, String>>,
        ) {
        }
    }

    #[test]
    fn enqueue_with_duplicate_id_overwrites_previous() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async {
            let queue: QueueToSaveWithId<u32, Obj> = QueueToSaveWithId::new("test");

            let captured = Arc::new(Mutex::new(Vec::<Obj>::new()));
            let notify = Arc::new(tokio::sync::Notify::new());
            let handler = Arc::new(CapturingHandler {
                captured: captured.clone(),
                notify: notify.clone(),
            });

            queue.register_events_handler(handler);

            queue.enqueue_single(Obj { id: 1, value: "a" });
            queue.enqueue_single(Obj { id: 2, value: "b" });
            queue.enqueue_single(Obj { id: 1, value: "c" });

            queue.start(Arc::new(NoopLogger));

            notify.notified().await;

            let guard = captured.lock().await;
            assert_eq!(guard.len(), 2);

            let one = guard.iter().find(|o| o.id == 1).expect("id=1 missing");
            assert_eq!(one.value, "c");

            let two = guard.iter().find(|o| o.id == 2).expect("id=2 missing");
            assert_eq!(two.value, "b");
        });
    }
}
