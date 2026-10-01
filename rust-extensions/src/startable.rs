/// Something that is brought to life by a single `start()` call - a queue, an
/// events loop, a background executor.
///
/// Everything `start` needs is handed over to `new`, so the things to start can
/// be collected as `Arc<dyn Startable + Send + Sync + 'static>` while the
/// application is being wired up and then started all together.
///
/// Every implementor has the very same `start()` as an inherent method as well -
/// the trait one just calls it - so starting a single thing needs no import.
pub trait Startable {
    fn start(&self);
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    use parking_lot::Mutex;

    use crate::background_executor::{BackgroundExecutor, BackgroundJob};
    use crate::background_executor_with_multi_threads::{
        BackgroundExecutorWithMultiThreads, BackgroundJobWithMultiThreads,
    };
    use crate::events_loop::{EventsLoop, EventsLoopTick};
    use crate::{
        AppStates, Logger, PersistObjectId, QueueToSave, QueueToSaveAsBulk,
        QueueToSaveAsBulkEventsHandler, QueueToSaveEventsHandler, QueueToSaveOrDeleteWithId,
        QueueToSaveOrDeleteWithIdEventsHandler, QueueToSaveWithId, QueueToSaveWithIdEventsHandler,
        UpsertOrDelete,
    };

    use super::Startable;

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
    }

    struct TestLogger;

    impl Logger for TestLogger {
        fn write_info(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
        fn write_warning(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
        fn write_error(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
        fn write_fatal_error(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
        fn write_debug_info(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
    }

    struct Item(u32);

    impl PersistObjectId<u32> for Item {
        fn get_persist_object_id(&self) -> &u32 {
            &self.0
        }
    }

    /// The one handler of everything which is started here - it writes down who
    /// was served, so it is seen that not a single one was left behind.
    struct Recorder {
        served: Arc<Mutex<Vec<&'static str>>>,
    }

    #[async_trait::async_trait]
    impl QueueToSaveEventsHandler<u32> for Recorder {
        async fn execute(&self, _item: u32) {
            self.served.lock().push("QueueToSave");
        }
    }

    #[async_trait::async_trait]
    impl QueueToSaveAsBulkEventsHandler<u32> for Recorder {
        async fn execute(&self, _items: &[u32], _attempt_no: usize) {
            self.served.lock().push("QueueToSaveAsBulk");
        }
    }

    #[async_trait::async_trait]
    impl QueueToSaveWithIdEventsHandler<Item> for Recorder {
        async fn execute(&self, _items: &[Item], _attempt_no: usize) {
            self.served.lock().push("QueueToSaveWithId");
        }
    }

    #[async_trait::async_trait]
    impl QueueToSaveOrDeleteWithIdEventsHandler<u32, Item> for Recorder {
        async fn execute(&self, _items: &[UpsertOrDelete<u32, Item>], _attempt_no: usize) {
            self.served.lock().push("QueueToSaveOrDeleteWithId");
        }
    }

    #[async_trait::async_trait]
    impl EventsLoopTick<u32> for Recorder {
        async fn started(&self) {}

        async fn tick(&self, _model: u32) -> crate::events_loop::RepeatIteration<u32> {
            self.served.lock().push("EventsLoop");
            crate::events_loop::RepeatIteration::No
        }

        async fn finished(&self) {}
    }

    #[async_trait::async_trait]
    impl BackgroundJob for Recorder {
        async fn execute(&self) -> crate::background_executor::RepeatIteration {
            self.served.lock().push("BackgroundExecutor");
            crate::background_executor::RepeatIteration::No
        }
    }

    #[async_trait::async_trait]
    impl BackgroundJobWithMultiThreads<u32> for Recorder {
        async fn execute(&self, _thread_id: &u32) -> crate::background_executor::RepeatIteration {
            self.served.lock().push("BackgroundExecutorWithMultiThreads");
            crate::background_executor::RepeatIteration::No
        }
    }

    async fn wait_for(served: &Arc<Mutex<Vec<&'static str>>>, expected: usize) {
        for _ in 0..2000 {
            if served.lock().len() >= expected {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        panic!(
            "Expected {} to be served, got {:?}",
            expected,
            served.lock().as_slice()
        );
    }

    #[test]
    fn everything_collected_is_started_through_the_trait() {
        rt().block_on(async {
            let served = Arc::new(Mutex::new(Vec::new()));
            let recorder = Arc::new(Recorder {
                served: served.clone(),
            });
            let logger: Arc<dyn Logger + Send + Sync + 'static> = Arc::new(TestLogger);

            let queue = Arc::new(QueueToSave::<u32>::new("queue", logger.clone()));
            queue.register_events_handler(recorder.clone());

            let queue_as_bulk =
                Arc::new(QueueToSaveAsBulk::<u32>::new("queue-as-bulk", logger.clone()));
            queue_as_bulk.register_events_handler(recorder.clone());

            let queue_with_id = Arc::new(QueueToSaveWithId::<u32, Item>::new(
                "queue-with-id",
                logger.clone(),
            ));
            queue_with_id.register_events_handler(recorder.clone());

            let queue_or_delete = Arc::new(QueueToSaveOrDeleteWithId::<u32, Item>::new(
                "queue-or-delete",
                logger.clone(),
            ));
            queue_or_delete.register_events_handler(recorder.clone());

            let events_loop = Arc::new(EventsLoop::<u32>::new(
                "events-loop",
                Arc::new(AppStates::create_initialized()),
                logger.clone(),
            ));
            events_loop.register_event_loop(recorder.clone());

            let executor = Arc::new(BackgroundExecutor::new("executor", logger.clone()));
            executor.register(recorder.clone());

            let multi_threads = Arc::new(BackgroundExecutorWithMultiThreads::<u32>::new(
                "multi-threads",
                logger.clone(),
            ));
            multi_threads.register(recorder.clone());

            // Collected while the application is being wired up...
            let to_start: Vec<Arc<dyn Startable + Send + Sync + 'static>> = vec![
                queue.clone(),
                queue_as_bulk.clone(),
                queue_with_id.clone(),
                queue_or_delete.clone(),
                events_loop.clone(),
                executor.clone(),
                multi_threads.clone(),
            ];

            // ...and started all together.
            for itm in &to_start {
                itm.start();
            }

            queue.enqueue_single(1);
            queue_as_bulk.enqueue_single(1);
            queue_with_id.enqueue_single(Item(1));
            queue_or_delete.enqueue_single(Item(1));
            events_loop.send(1);
            executor.trigger();
            multi_threads.trigger(1);

            wait_for(&served, 7).await;
            tokio::time::sleep(Duration::from_millis(50)).await;

            let mut served = served.lock().clone();
            served.sort();

            assert_eq!(
                served,
                [
                    "BackgroundExecutor",
                    "BackgroundExecutorWithMultiThreads",
                    "EventsLoop",
                    "QueueToSave",
                    "QueueToSaveAsBulk",
                    "QueueToSaveOrDeleteWithId",
                    "QueueToSaveWithId",
                ]
            );
        });
    }
}
