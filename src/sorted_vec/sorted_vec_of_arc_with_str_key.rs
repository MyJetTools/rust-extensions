use std::sync::Arc;

use crate::sorted_vec::{EntityWithStrKey, GetOrCreateEntry, InsertEntity};

use super::InsertIfNotExists;

#[derive(Clone, Debug, Default)]
pub struct SortedVecOfArcWithStrKey<TValue: EntityWithStrKey> {
    items: Vec<Arc<TValue>>,
}

impl<TValue: EntityWithStrKey> SortedVecOfArcWithStrKey<TValue> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn new_with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
        }
    }

    pub fn capacity(&self) -> usize {
        self.items.capacity()
    }

    pub fn reserve(&mut self, capacity: usize) {
        self.items.reserve(capacity);
    }

    pub fn reserve_exact(&mut self, capacity: usize) {
        self.items.reserve_exact(capacity);
    }

    // Returns the index of the inserted item and old item if it was replaced
    pub fn insert_or_replace(&mut self, item: Arc<TValue>) -> (usize, Option<Arc<TValue>>) {
        let insert_index = self
            .items
            .binary_search_by(|itm| itm.get_key().cmp(item.get_key()));

        match insert_index {
            Ok(index) => {
                let got = std::mem::replace(&mut self.items[index], item);
                (index, Some(got))
            }
            Err(index) => {
                self.items.insert(index, item);
                (index, None)
            }
        }
    }

    pub fn insert_or_if_not_exists<'s>(
        &'s mut self,
        key: &str,
    ) -> InsertIfNotExists<'s, Arc<TValue>> {
        let insert_index = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match insert_index {
            Ok(index) => {
                return InsertIfNotExists::Exists(index);
            }
            Err(index) => {
                return InsertIfNotExists::Insert(InsertEntity::new(index, &mut self.items));
            }
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));
        result.is_ok()
    }

    pub fn get(&self, key: &str) -> Option<&Arc<TValue>> {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => self.items.get(index),
            Err(_) => None,
        }
    }

    pub fn get_or_create<'s>(&'s mut self, key: &str) -> GetOrCreateEntry<'s, Arc<TValue>> {
        let index = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match index {
            Ok(index) => GetOrCreateEntry::Get(&self.items[index]),
            Err(index) => GetOrCreateEntry::Create(InsertEntity::new(index, &mut self.items)),
        }
    }

    pub fn get_by_index(&self, index: usize) -> Option<&Arc<TValue>> {
        self.items.get(index)
    }

    pub fn get_from_key_to_up(&self, key: &str) -> &[Arc<TValue>] {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => &self.items[index..],
            Err(index) => &self.items[index..],
        }
    }

    pub fn get_from_bottom_to_key(&self, key: &str) -> &[Arc<TValue>] {
        if self.items.is_empty() {
            return &self.items[0..0];
        }

        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => &self.items[..=index],
            Err(index) => &self.items[..index],
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<Arc<TValue>> {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => Some(self.items.remove(index)),
            Err(_) => None,
        }
    }

    pub fn remove_at(&mut self, index: usize) -> Option<Arc<TValue>> {
        if index >= self.items.len() {
            return None;
        }

        Some(self.items.remove(index))
    }

    pub fn clear(&mut self, max_capacity: Option<usize>) {
        self.items.clear();

        if let Some(max_capacity) = max_capacity {
            self.items.reserve(max_capacity);
        }
    }

    pub fn first(&self) -> Option<&Arc<TValue>> {
        self.items.first()
    }

    pub fn last(&self) -> Option<&Arc<TValue>> {
        self.items.last()
    }

    pub fn iter<'s>(&'s self) -> std::slice::Iter<'s, Arc<TValue>> {
        self.items.iter()
    }

    pub fn iter_mut<'s>(&'s mut self) -> std::slice::IterMut<'s, Arc<TValue>> {
        self.items.iter_mut()
    }

    pub fn into_vec(self) -> Vec<Arc<TValue>> {
        self.items
    }

    pub fn to_vec_cloned(&self) -> Vec<Arc<TValue>> {
        self.items.clone()
    }

    pub fn as_slice(&self) -> &[Arc<TValue>] {
        self.items.as_slice()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn range(&self, range: std::ops::Range<&str>) -> &[Arc<TValue>] {
        let index_from = self
            .items
            .binary_search_by(|itm| itm.get_key().cmp(&range.start));

        let index_from = match index_from {
            Ok(index) => index,
            Err(index) => index,
        };

        let index_to = self
            .items
            .binary_search_by(|itm| itm.get_key().cmp(&range.end));

        match index_to {
            Ok(index_to) => {
                return &self.items[index_from..=index_to];
            }
            Err(index_to) => &self.items[index_from..index_to],
        }
    }

    pub fn drain_into_vec(&mut self) -> Vec<Arc<TValue>> {
        let mut result = Vec::with_capacity(self.items.len());
        while let Some(item) = self.items.pop() {
            result.push(item);
        }
        result
    }

    pub fn pop(&mut self) -> Option<Arc<TValue>> {
        self.items.pop()
    }
}
