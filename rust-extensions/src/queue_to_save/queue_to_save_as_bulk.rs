use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{queue_to_save::inner_as_bulk::QueueToSaveInnerAsBulk, Logger, StrOrString};

enum HandlerStatus<T> {
    None,
    Some(Arc<dyn QueueToSaveAsBulkEventsHandler<T> + Send + Sync + 'static>),
    Working,
}

pub struct QueueToSaveAsBulk<T: Send + Sync + 'static> {
    inner: Arc<QueueToSaveInnerAsBulk<T>>,
    handler: Mutex<HandlerStatus<T>>,
}

impl<T: Send + Sync + 'static> QueueToSaveAsBulk<T> {
    pub fn new(name: impl Into<StrOrString<'static>>) -> Self {
        Self {
            inner: Arc::new(QueueToSaveInnerAsBulk::new(name.into())),
            handler: Mutex::new(HandlerStatus::None),
        }
    }
    pub async fn enqueue(&self, items: impl Iterator<Item = T>) {
        self.inner.enqueue(items).await;
    }

    pub async fn enqueue_single(&self, item: T) {
        self.inner.enqueue_single(item).await;
    }

    pub async fn register_events_handler(
        &self,
        events_handle: Arc<dyn QueueToSaveAsBulkEventsHandler<T> + Send + Sync + 'static>,
    ) {
        let mut write_access = self.handler.lock().await;
        *write_access = HandlerStatus::Some(events_handle);
    }

    pub fn get_name(&self) -> &str {
        self.inner.name.as_str()
    }

    pub async fn start(&self, logger: Arc<dyn Logger + Send + Sync + 'static>) {
        let mut write_access = self.handler.lock().await;

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
pub trait QueueToSaveAsBulkEventsHandler<T: Send + Sync + 'static> {
    async fn execute(&self, items: Vec<T>);
}

async fn queue_to_save_loop<T: Send + Sync + 'static>(
    inner: Arc<QueueToSaveInnerAsBulk<T>>,
    handler: Arc<dyn QueueToSaveAsBulkEventsHandler<T> + Send + Sync + 'static>,
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
