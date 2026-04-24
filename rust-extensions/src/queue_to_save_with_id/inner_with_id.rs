use std::{collections::HashMap, hash::Hash, time::Duration};

use parking_lot::Mutex;

use crate::StrOrString;

use super::async_waker::*;
use super::persist_object_id::PersistObjectId;

pub struct QueueToSaveInnerWithId<ID, T>
where
    ID: Hash + Eq + Clone,
    T: PersistObjectId<ID>,
{
    queue: Mutex<(HashMap<ID, T>, AsyncWaker)>,

    pub(crate) max_chunk_size: usize,
    pub(crate) timeout: Duration,
    pub(crate) name: StrOrString<'static>,
}

impl<ID, T> QueueToSaveInnerWithId<ID, T>
where
    ID: Hash + Eq + Clone,
    T: PersistObjectId<ID>,
{
    pub fn new(name: StrOrString<'static>) -> Self {
        Self {
            queue: Mutex::new((HashMap::new(), AsyncWaker::default())),
            max_chunk_size: 50,
            timeout: Duration::from_secs(10),
            name,
        }
    }

    pub(crate) fn enqueue(&self, items: impl Iterator<Item = T>) {
        let mut queue = self.queue.lock();
        for item in items {
            let id = item.get_persist_object_id().clone();
            queue.0.insert(id, item);
        }
        queue.1.wake();
    }

    pub(crate) fn enqueue_single(&self, item: T) {
        let mut queue = self.queue.lock();
        let id = item.get_persist_object_id().clone();
        queue.0.insert(id, item);
        queue.1.wake();
    }

    pub(crate) async fn dequeue(&self) -> Vec<T> {
        loop {
            match self.try_dequeue() {
                Ok(values) => {
                    return values;
                }
                Err(err) => {
                    err.await_me().await;
                }
            }
        }
    }

    fn try_dequeue(&self) -> Result<Vec<T>, AsyncWakerAwaiter> {
        let mut write_access = self.queue.lock();

        if write_access.0.is_empty() {
            return Err(write_access.1.get_awaiter());
        }

        if write_access.0.len() <= self.max_chunk_size {
            return Ok(std::mem::take(&mut write_access.0).into_values().collect());
        }

        let keys: Vec<ID> = write_access
            .0
            .keys()
            .take(self.max_chunk_size)
            .cloned()
            .collect();

        let mut result = Vec::with_capacity(keys.len());
        for key in keys {
            if let Some(item) = write_access.0.remove(&key) {
                result.push(item);
            }
        }

        Ok(result)
    }
}
