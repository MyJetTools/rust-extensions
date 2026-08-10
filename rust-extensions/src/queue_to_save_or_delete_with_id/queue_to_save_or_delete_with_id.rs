use std::{hash::Hash, sync::Arc};

use parking_lot::Mutex;

use crate::{Logger, StrOrString};

use super::{
    inner_or_delete_with_id::QueueToSaveOrDeleteInnerWithId, upsert_or_delete::UpsertOrDelete,
    PersistObjectId,
};

enum HandlerStatus<ID, T> {
    None,
    Some(Arc<dyn QueueToSaveOrDeleteWithIdEventsHandler<ID, T> + Send + Sync + 'static>),
    Working,
}

pub struct QueueToSaveOrDeleteWithId<ID, T>
where
    ID: Hash + Eq + Clone + Send + Sync + 'static,
    T: PersistObjectId<ID> + Send + Sync + 'static,
{
    inner: Arc<QueueToSaveOrDeleteInnerWithId<ID, T>>,
    handler: Mutex<HandlerStatus<ID, T>>,
}

impl<ID, T> QueueToSaveOrDeleteWithId<ID, T>
where
    ID: Hash + Eq + Clone + Send + Sync + 'static,
    T: PersistObjectId<ID> + Send + Sync + 'static,
{
    pub fn new(name: impl Into<StrOrString<'static>>) -> Self {
        Self {
            inner: Arc::new(QueueToSaveOrDeleteInnerWithId::new(name.into())),
            handler: Mutex::new(HandlerStatus::None),
        }
    }

    pub fn enqueue(&self, items: impl Iterator<Item = T>) {
        self.inner.enqueue(items);
    }

    pub fn enqueue_single(&self, item: T) {
        self.inner.enqueue_single(item);
    }

    /// Marks the object with the given ID as the one to be deleted.
    ///
    /// A pending upsert of the same ID - if any - is dropped right here: there is no
    /// point in saving an object which is about to be deleted.
    pub fn enqueue_delete(&self, id: ID) {
        self.inner.enqueue_delete(id);
    }

    pub fn enqueue_delete_multiple(&self, ids: impl Iterator<Item = ID>) {
        self.inner.enqueue_delete_multiple(ids);
    }

    pub fn register_events_handler(
        &self,
        events_handle: Arc<dyn QueueToSaveOrDeleteWithIdEventsHandler<ID, T> + Send + Sync + 'static>,
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
                    "Event handler is not registered in QueueToSaveOrDeleteWithId {}",
                    self.inner.name
                );
            }
            HandlerStatus::Some(handler) => {
                tokio::spawn(queue_to_save_or_delete_with_id_loop(
                    self.inner.clone(),
                    handler.clone(),
                    logger,
                ));
            }
            HandlerStatus::Working => {
                panic!(
                    "QueueToSaveOrDeleteWithId {} is already started",
                    self.inner.name
                );
            }
        }

        *write_access = HandlerStatus::Working;
    }
}

#[async_trait::async_trait]
pub trait QueueToSaveOrDeleteWithIdEventsHandler<ID: Send + Sync + 'static, T: Send + Sync + 'static>
{
    async fn execute(&self, items: Vec<UpsertOrDelete<ID, T>>);
}

async fn queue_to_save_or_delete_with_id_loop<ID, T>(
    inner: Arc<QueueToSaveOrDeleteInnerWithId<ID, T>>,
    handler: Arc<dyn QueueToSaveOrDeleteWithIdEventsHandler<ID, T> + Send + Sync + 'static>,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
) where
    ID: Hash + Eq + Clone + Send + Sync + 'static,
    T: PersistObjectId<ID> + Send + Sync + 'static,
{
    println!(
        "QueueToSaveOrDeleteWithId {} is started",
        inner.name.as_str()
    );
    let timeout = inner.timeout;
    loop {
        let events = inner.dequeue().await;

        let handler = handler.clone();
        let feature = tokio::spawn(async move {
            let future = handler.execute(events);

            tokio::time::timeout(timeout, future).await
        });

        let result = match feature.await {
            Ok(value) => value,
            Err(_) => {
                let msg = format!(
                    "Panic at QueueToSaveOrDeleteWithIdEventsHandler named {}",
                    inner.name.as_str()
                );

                logger.write_error(
                    "QueueToSaveOrDeleteWithId.loop".to_string(),
                    msg,
                    None.into(),
                );
                continue;
            }
        };

        if result.is_err() {
            let msg = format!(
                "Timeout {:?} at QueueToSaveOrDeleteWithIdEventsHandler named {}",
                inner.timeout,
                inner.name.as_str()
            );

            logger.write_error(
                "QueueToSaveOrDeleteWithId.loop".to_string(),
                msg,
                None.into(),
            );
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
        captured: Arc<Mutex<Vec<UpsertOrDelete<u32, Obj>>>>,
        notify: Arc<tokio::sync::Notify>,
    }

    #[async_trait::async_trait]
    impl QueueToSaveOrDeleteWithIdEventsHandler<u32, Obj> for CapturingHandler {
        async fn execute(&self, items: Vec<UpsertOrDelete<u32, Obj>>) {
            let mut guard = self.captured.lock().await;
            guard.extend(items);
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
    fn delete_replaces_pending_upsert_and_is_delivered_as_id() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async {
            let queue: QueueToSaveOrDeleteWithId<u32, Obj> = QueueToSaveOrDeleteWithId::new("test");

            let captured = Arc::new(Mutex::new(Vec::new()));
            let notify = Arc::new(tokio::sync::Notify::new());
            let handler = Arc::new(CapturingHandler {
                captured: captured.clone(),
                notify: notify.clone(),
            });

            queue.register_events_handler(handler);

            queue.enqueue_single(Obj { id: 1, value: "a" });
            queue.enqueue_single(Obj { id: 2, value: "b" });
            queue.enqueue_single(Obj { id: 1, value: "c" });
            queue.enqueue_delete(1);
            queue.enqueue_delete(3);

            queue.start(Arc::new(NoopLogger));

            notify.notified().await;

            let guard = captured.lock().await;
            assert_eq!(guard.len(), 3);

            let one = guard.iter().find(|itm| *itm.get_id() == 1).unwrap();
            assert!(one.is_delete());

            let two = guard.iter().find(|itm| *itm.get_id() == 2).unwrap();
            assert_eq!(two.unwrap_as_upsert().value, "b");

            let three = guard.iter().find(|itm| *itm.get_id() == 3).unwrap();
            assert!(three.is_delete());
        });
    }
}
