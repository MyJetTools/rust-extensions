use std::{collections::HashMap, hash::Hash, time::Duration};

use parking_lot::Mutex;

use crate::StrOrString;

use super::async_waker::*;
use super::upsert_or_delete::UpsertOrDelete;
use super::PersistObjectId;

/// What is pending for a certain ID.
///
/// `Delete` holds nothing - the ID is the map key, and the object itself is dropped
/// as soon as the delete is enqueued.
enum PendingState<T> {
    Upsert(T),
    Delete,
}

pub struct QueueToSaveOrDeleteInnerWithId<ID, T>
where
    ID: Hash + Eq + Clone,
    T: PersistObjectId<ID>,
{
    queue: Mutex<(HashMap<ID, PendingState<T>>, AsyncWaker)>,

    pub(crate) max_chunk_size: usize,
    pub(crate) timeout: Duration,
    pub(crate) name: StrOrString<'static>,
}

impl<ID, T> QueueToSaveOrDeleteInnerWithId<ID, T>
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
            queue.0.insert(id, PendingState::Upsert(item));
        }
        queue.1.wake();
    }

    pub(crate) fn enqueue_single(&self, item: T) {
        let mut queue = self.queue.lock();
        let id = item.get_persist_object_id().clone();
        queue.0.insert(id, PendingState::Upsert(item));
        queue.1.wake();
    }

    pub(crate) fn enqueue_delete(&self, id: ID) {
        let mut queue = self.queue.lock();
        queue.0.insert(id, PendingState::Delete);
        queue.1.wake();
    }

    pub(crate) fn enqueue_delete_multiple(&self, ids: impl Iterator<Item = ID>) {
        let mut queue = self.queue.lock();
        for id in ids {
            queue.0.insert(id, PendingState::Delete);
        }
        queue.1.wake();
    }

    pub(crate) async fn dequeue(&self) -> Vec<UpsertOrDelete<ID, T>> {
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

    fn try_dequeue(&self) -> Result<Vec<UpsertOrDelete<ID, T>>, AsyncWakerAwaiter> {
        let mut write_access = self.queue.lock();

        if write_access.0.is_empty() {
            return Err(write_access.1.get_awaiter());
        }

        if write_access.0.len() <= self.max_chunk_size {
            let result = std::mem::take(&mut write_access.0)
                .into_iter()
                .map(|(id, state)| state.into_upsert_or_delete(id))
                .collect();

            return Ok(result);
        }

        let keys: Vec<ID> = write_access
            .0
            .keys()
            .take(self.max_chunk_size)
            .cloned()
            .collect();

        let mut result = Vec::with_capacity(keys.len());
        for key in keys {
            if let Some(state) = write_access.0.remove(&key) {
                result.push(state.into_upsert_or_delete(key));
            }
        }

        Ok(result)
    }
}

impl<T> PendingState<T> {
    fn into_upsert_or_delete<ID>(self, id: ID) -> UpsertOrDelete<ID, T> {
        match self {
            Self::Upsert(value) => UpsertOrDelete::Upsert(value),
            Self::Delete => UpsertOrDelete::Delete(id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Obj {
        id: u32,
        value: &'static str,
    }

    impl PersistObjectId<u32> for Obj {
        fn get_persist_object_id(&self) -> &u32 {
            &self.id
        }
    }

    fn create_queue() -> QueueToSaveOrDeleteInnerWithId<u32, Obj> {
        QueueToSaveOrDeleteInnerWithId::new("test".into())
    }

    #[test]
    fn delete_replaces_pending_upsert() {
        let queue = create_queue();

        queue.enqueue_single(Obj { id: 1, value: "a" });
        queue.enqueue_single(Obj { id: 2, value: "b" });
        queue.enqueue_delete(1);

        let mut result = queue.try_dequeue().ok().unwrap();
        result.sort_by_key(|itm| *itm.get_id());

        assert_eq!(result.len(), 2);

        assert_eq!(*result[0].unwrap_as_delete(), 1);
        assert_eq!(result[1].unwrap_as_upsert().value, "b");
    }

    #[test]
    fn upsert_replaces_pending_delete() {
        let queue = create_queue();

        queue.enqueue_delete(1);
        queue.enqueue_single(Obj { id: 1, value: "a" });

        let result = queue.try_dequeue().ok().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].unwrap_as_upsert().value, "a");
    }

    #[test]
    fn delete_of_unknown_id_is_still_delivered() {
        let queue = create_queue();

        queue.enqueue_delete(5);

        let result = queue.try_dequeue().ok().unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(*result[0].unwrap_as_delete(), 5);
    }

    #[test]
    fn empty_queue_returns_awaiter() {
        let queue = create_queue();
        assert!(queue.try_dequeue().is_err());
    }

    #[test]
    fn dequeue_is_limited_by_max_chunk_size() {
        let mut queue = create_queue();
        queue.max_chunk_size = 2;

        queue.enqueue((0..5).map(|id| Obj { id, value: "a" }));
        queue.enqueue_delete(3);

        let chunk = queue.try_dequeue().ok().unwrap();
        assert_eq!(chunk.len(), 2);

        let chunk = queue.try_dequeue().ok().unwrap();
        assert_eq!(chunk.len(), 2);

        let chunk = queue.try_dequeue().ok().unwrap();
        assert_eq!(chunk.len(), 1);

        assert!(queue.try_dequeue().is_err());
    }

    #[test]
    fn split_separates_upserts_from_deletes() {
        let queue = create_queue();

        queue.enqueue((0..4).map(|id| Obj { id, value: "a" }));
        queue.enqueue_delete(1);
        queue.enqueue_delete(3);

        let (mut to_upsert, mut to_delete) =
            UpsertOrDelete::split(queue.try_dequeue().ok().unwrap());

        to_upsert.sort_by_key(|itm| itm.id);
        to_delete.sort();

        assert_eq!(to_upsert.iter().map(|itm| itm.id).collect::<Vec<_>>(), [0, 2]);
        assert_eq!(to_delete, [1, 3]);
    }
}
