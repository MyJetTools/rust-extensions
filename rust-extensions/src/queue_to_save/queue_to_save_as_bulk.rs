use std::{panic::AssertUnwindSafe, sync::Arc, time::Duration};

use futures::FutureExt;
use parking_lot::Mutex;

use crate::{queue_to_save::inner_as_bulk::QueueToSaveInnerAsBulk, Logger, StrOrString};

enum HandlerStatus<T> {
    None,
    Some(Arc<dyn QueueToSaveAsBulkEventsHandler<T> + Send + Sync + 'static>),
    Working,
}

pub struct QueueToSaveAsBulk<T: Send + Sync + 'static> {
    inner: Arc<QueueToSaveInnerAsBulk<T>>,
    handler: Mutex<HandlerStatus<T>>,
    retry_timeout: Duration,
}

impl<T: Send + Sync + 'static> QueueToSaveAsBulk<T> {
    pub fn new(name: impl Into<StrOrString<'static>>) -> Self {
        Self {
            inner: Arc::new(QueueToSaveInnerAsBulk::new(name.into())),
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
        events_handle: Arc<dyn QueueToSaveAsBulkEventsHandler<T> + Send + Sync + 'static>,
    ) {
        let mut write_access = self.handler.lock();
        *write_access = HandlerStatus::Some(events_handle);
    }

    pub fn get_name(&self) -> &str {
        self.inner.name.as_str()
    }

    /// Amount of the items which are waiting in the queue right now.
    ///
    /// The chunk which is being handled at the moment is already dequeued - it
    /// is not counted here.
    pub fn queue_len(&self) -> usize {
        self.inner.queue_len()
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
                    self.retry_timeout,
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
    /// Returning means the items are saved. A panic or a timeout means they are
    /// not - the very same items are handed over again after `retry_timeout`,
    /// until it returns.
    ///
    /// `attempt_no` is 0 on the first attempt and grows by one on every retry of
    /// the same items.
    async fn execute(&self, items: &[T], attempt_no: usize);
}

async fn queue_to_save_loop<T: Send + Sync + 'static>(
    inner: Arc<QueueToSaveInnerAsBulk<T>>,
    handler: Arc<dyn QueueToSaveAsBulkEventsHandler<T> + Send + Sync + 'static>,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
    retry_timeout: Duration,
) {
    println!("Queue to save {} is started", inner.name.as_str());
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
                Ok(Err(_)) => format!(
                    "Panic at QueueToSaveEventsHandler named {}. {} item(s) are going to be saved again",
                    inner.name.as_str(),
                    events.len()
                ),
                Err(_) => format!(
                    "Timeout {:?} at QueueToSaveEventsHandler named {}. {} item(s) are going to be saved again",
                    timeout,
                    inner.name.as_str(),
                    events.len()
                ),
            };

            logger.write_error("QueueToSave.loop".to_string(), msg, None.into());
            attempt_no += 1;

            tokio::time::sleep(retry_timeout).await;
        }
    }
}
