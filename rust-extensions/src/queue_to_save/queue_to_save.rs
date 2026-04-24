use std::sync::Arc;

use parking_lot::Mutex;

use crate::{queue_to_save::inner_as_single::QueueToSaveInnerAsSingle, Logger, StrOrString};

enum HandlerStatus<T> {
    None,
    Some(Arc<dyn QueueToSaveEventsHandler<T> + Send + Sync + 'static>),
    Working,
}

pub struct QueueToSave<T: Send + Sync + 'static> {
    inner: Arc<QueueToSaveInnerAsSingle<T>>,
    handler: Mutex<HandlerStatus<T>>,
}

impl<T: Send + Sync + 'static> QueueToSave<T> {
    pub fn new(name: impl Into<StrOrString<'static>>) -> Self {
        Self {
            inner: Arc::new(QueueToSaveInnerAsSingle::new(name.into())),
            handler: Mutex::new(HandlerStatus::None),
        }
    }
    pub fn enqueue(&self, items: impl Iterator<Item = T>) {
        self.inner.enqueue(items);
    }

    pub fn enqueue_single(&self, item: T) {
        self.inner.enqueue_single(item);
    }

    pub fn register_events_handler(
        &self,
        events_handle: Arc<dyn QueueToSaveEventsHandler<T> + Send + Sync + 'static>,
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
                    "Event handler is not registered in QueueToSave {}",
                    self.inner.name
                );
            }
            HandlerStatus::Some(handler) => {
                tokio::spawn(queue_to_save_loop(
                    self.inner.clone(),
                    handler.clone(),
                    logger,
                ));
            }
            HandlerStatus::Working => {
                panic!("QueueToSave {} is already started", self.inner.name);
            }
        }

        *write_access = HandlerStatus::Working;
    }
}

#[async_trait::async_trait]
pub trait QueueToSaveEventsHandler<T: Send + Sync + 'static> {
    async fn execute(&self, items: T);
}

async fn queue_to_save_loop<T: Send + Sync + 'static>(
    inner: Arc<QueueToSaveInnerAsSingle<T>>,
    handler: Arc<dyn QueueToSaveEventsHandler<T> + Send + Sync + 'static>,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
) {
    println!("Queue to save {} is started", inner.name.as_str());
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
                    "Panic at QueueToSaveEventsHandler named {}",
                    inner.name.as_str()
                );

                logger.write_error("QueueToSave.loop".to_string(), msg, None.into());
                continue;
            }
        };

        if result.is_err() {
            let msg = format!(
                "Timeout {:?} at QueueToSaveEventsHandler named {}",
                inner.timeout,
                inner.name.as_str()
            );

            logger.write_error("QueueToSave.loop".to_string(), msg, None.into());
        }
    }
}
