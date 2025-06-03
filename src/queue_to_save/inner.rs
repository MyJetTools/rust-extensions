use std::time::Duration;

use tokio::sync::Mutex;

use crate::{queue_to_save::async_waker::*, StrOrString};

pub struct QueueToSaveInner<T> {
    queue: Mutex<(Vec<T>, AsyncWaker)>,

    pub(crate) max_chunk_size: usize,
    pub(crate) timeout: Duration,
    pub(crate) name: StrOrString<'static>,
}

impl<T> QueueToSaveInner<T> {
    pub fn new(name: StrOrString<'static>) -> Self {
        Self {
            queue: Default::default(),
            max_chunk_size: 50,
            timeout: Duration::from_secs(10),
            name,
        }
    }
    pub(crate) async fn enqueue(&self, items: impl Iterator<Item = T>) {
        let mut queue = self.queue.lock().await;
        println!("Enqueuing items");
        queue.0.extend(items);
        queue.1.wake();
    }

    pub(crate) async fn dequeue(&self) -> Vec<T> {
        loop {
            match self.try_dequeue().await {
                Ok(values) => {
                    println!("Got {} elements", values.len());
                    return values;
                }
                Err(err) => {
                    println!("Start awaiting element");
                    err.await_me().await;
                }
            }
        }
    }

    async fn try_dequeue(&self) -> Result<Vec<T>, AsyncWakerAwaiter> {
        let mut write_access = self.queue.lock().await;

        if write_access.0.len() == 0 {
            return Err(write_access.1.get_awaiter());
        }

        if write_access.0.len() <= self.max_chunk_size {
            return Ok(std::mem::take(&mut write_access.0));
        }

        let mut result = Vec::with_capacity(self.max_chunk_size);

        while result.len() < self.max_chunk_size {
            if let Some(item) = write_access.0.pop() {
                result.push(item);
            } else {
                break;
            }
        }

        Ok(result)
    }
}
