use std::sync::Arc;

use crate::{queue_to_save::inner::QueueToSaveInner, Logger};

pub enum QueueToSaveStatus<T: Send + Sync + 'static> {
    Init(QueueToSaveInner<T>),
    Working(Arc<QueueToSaveInner<T>>),
}

impl<T: Send + Sync + 'static> QueueToSaveStatus<T> {
    pub fn get_inner(&self) -> &QueueToSaveInner<T> {
        match self {
            QueueToSaveStatus::Init(inner) => inner,
            QueueToSaveStatus::Working(inner) => inner,
        }
    }
}

pub struct QueueToSave<T: Send + Sync + 'static> {
    inner: Option<QueueToSaveStatus<T>>,
}

impl<T: Send + Sync + 'static> QueueToSave<T> {
    pub fn new(name: String) -> Self {
        Self {
            inner: Some(QueueToSaveStatus::Init(QueueToSaveInner::new(name))),
        }
    }
    pub async fn enqueue(&self, items: impl Iterator<Item = T>) {
        self.inner
            .as_ref()
            .unwrap()
            .get_inner()
            .enqueue(items)
            .await;
    }

    pub fn register_events_handler(
        mut self,
        events_handle: Arc<dyn QueueToSaveEventsHandler<T> + Send + Sync + 'static>,
    ) -> Self {
        match &mut self.inner.as_mut().unwrap() {
            QueueToSaveStatus::Init(inner) => {
                inner.handler = Some(events_handle);
            }
            QueueToSaveStatus::Working(_) => {
                panic!("Can not register events_handle to QueueToSave when it in working state");
            }
        }
        self
    }

    fn get_name(&self) -> &str {
        &self.inner.as_ref().unwrap().get_inner().name
    }

    pub fn start(&mut self, logger: Arc<dyn Logger + Send + Sync + 'static>) {
        match self.inner.take().unwrap() {
            QueueToSaveStatus::Init(inner) => {
                let inner = Arc::new(inner);
                tokio::spawn(queue_to_save_loop(inner.clone(), logger));
                self.inner = Some(QueueToSaveStatus::Working(inner))
            }
            QueueToSaveStatus::Working(inner) => {
                self.inner = Some(QueueToSaveStatus::Working(inner));
                panic!("Queue to save {} is already started", self.get_name());
            }
        }
    }
}

#[async_trait::async_trait]
pub trait QueueToSaveEventsHandler<T: Send + Sync + 'static> {
    async fn execute(&self, items: Vec<T>);
}

async fn queue_to_save_loop<T: Send + Sync + 'static>(
    inner: Arc<QueueToSaveInner<T>>,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
) {
    let handler = inner.handler.clone().unwrap();
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
