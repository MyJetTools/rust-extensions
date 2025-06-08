use std::{collections::VecDeque, time::Duration};

use tokio::sync::Mutex;

use crate::{queue_to_save::async_waker::*, StrOrString};

pub struct QueueToSaveInnerAsSingle<T> {
    queue: Mutex<(VecDeque<T>, AsyncWaker)>,

    pub(crate) timeout: Duration,
    pub(crate) name: StrOrString<'static>,
}

impl<T> QueueToSaveInnerAsSingle<T> {
    pub fn new(name: StrOrString<'static>) -> Self {
        Self {
            queue: Default::default(),
            timeout: Duration::from_secs(10),
            name,
        }
    }
    pub(crate) async fn enqueue(&self, items: impl Iterator<Item = T>) {
        let mut queue = self.queue.lock().await;

        for itm in items {
            queue.0.push_back(itm);
        }

        queue.1.wake();
    }

    pub(crate) async fn enqueue_single(&self, item: T) {
        let mut queue = self.queue.lock().await;
        queue.0.push_back(item);
        queue.1.wake();
    }

    pub(crate) async fn dequeue(&self) -> T {
        loop {
            match self.try_dequeue().await {
                Ok(values) => {
                    return values;
                }
                Err(err) => {
                    err.await_me().await;
                }
            }
        }
    }

    async fn try_dequeue(&self) -> Result<T, AsyncWakerAwaiter> {
        let mut write_access = self.queue.lock().await;

        match write_access.0.pop_front() {
            Some(result) => Ok(result),
            None => Err(write_access.1.get_awaiter()),
        }
    }
}
