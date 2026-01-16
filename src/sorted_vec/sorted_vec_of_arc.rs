use std::sync::Arc;

use crate::sorted_vec::{GetOrCreateEntry, InsertEntity};

use super::{EntityWithKey, InsertIfNotExists};

#[derive(Clone, Debug, Default)]
pub struct SortedVecOfArc<TKey: Ord, TValue: EntityWithKey<TKey>> {
    items: Vec<Arc<TValue>>,
    itm: std::marker::PhantomData<TKey>,
}

impl<TKey: Ord, TValue: EntityWithKey<TKey>> SortedVecOfArc<TKey, TValue> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            itm: std::marker::PhantomData,
        }
    }

    pub fn new_with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            itm: std::marker::PhantomData,
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
        key: &TKey,
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

    pub fn contains(&self, key: &TKey) -> bool {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));
        result.is_ok()
    }

    pub fn get(&self, key: &TKey) -> Option<&Arc<TValue>> {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => self.items.get(index),
            Err(_) => None,
        }
    }

    pub fn get_or_create<'s>(&'s mut self, key: &TKey) -> GetOrCreateEntry<'s, Arc<TValue>> {
        let index = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match index {
            Ok(index) => GetOrCreateEntry::Get(&self.items[index]),
            Err(index) => GetOrCreateEntry::Create(InsertEntity::new(index, &mut self.items)),
        }
    }

    pub fn get_by_index(&self, index: usize) -> Option<&Arc<TValue>> {
        self.items.get(index)
    }

    pub fn get_from_key_to_up(&self, key: &TKey) -> &[Arc<TValue>] {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => &self.items[index..],
            Err(index) => &self.items[index..],
        }
    }

    pub fn get_from_bottom_to_key(&self, key: &TKey) -> &[Arc<TValue>] {
        if self.items.is_empty() {
            return &self.items[0..0];
        }

        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => &self.items[..=index],
            Err(index) => &self.items[..index],
        }
    }

    pub fn remove(&mut self, key: &TKey) -> Option<Arc<TValue>> {
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

    pub fn first(&self) -> Option<&Arc<TValue>> {
        self.items.first()
    }

    pub fn last(&self) -> Option<&Arc<TValue>> {
        self.items.last()
    }

    pub fn clear(&mut self, max_capacity: Option<usize>) {
        self.items.clear();

        if let Some(max_capacity) = max_capacity {
            self.items.reserve(max_capacity);
        }
    }

    pub fn sub_sequence(&self, range: std::ops::Range<TKey>) -> Self
    where
        TValue: Clone,
    {
        let items = self.range(range).to_vec();
        Self {
            items,
            itm: std::marker::PhantomData,
        }
    }

    pub fn range_by_index<'s, R: std::ops::RangeBounds<usize>>(
        &'s self,
        range: R,
    ) -> SortedVecOfArcSlice<'s, TKey, TValue>
    where
        TValue: Clone,
    {
        let len = self.items.len();
        let start = match range.start_bound() {
            std::ops::Bound::Included(&value) => value,
            std::ops::Bound::Excluded(&value) => value.saturating_add(1),
            std::ops::Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            std::ops::Bound::Included(&value) => value.saturating_add(1),
            std::ops::Bound::Excluded(&value) => value,
            std::ops::Bound::Unbounded => len,
        };

        let start = start.min(len);
        let end = end.min(len);
        if start >= end {
            return SortedVecOfArcSlice::new(&self.items[0..0]);
        }

        SortedVecOfArcSlice::new(&self.items[start..end])
    }

    pub fn truncate_capacity(&mut self, capacity: usize) {
        self.items.truncate(capacity);
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

    pub fn drain_into_vec(&mut self) -> Vec<Arc<TValue>> {
        let mut result = Vec::with_capacity(self.items.len());
        while let Some(item) = self.items.pop() {
            result.push(item);
        }
        result
    }

    pub fn range<'s>(
        &'s self,
        range: std::ops::Range<TKey>,
    ) -> SortedVecOfArcSlice<'s, TKey, TValue>
    where
        TValue: Clone,
    {
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
                return SortedVecOfArcSlice::new(&self.items[index_from..=index_to]);
            }
            Err(index_to) => SortedVecOfArcSlice::new(&self.items[index_from..index_to]),
        }
    }

    pub fn pop(&mut self) -> Option<Arc<TValue>> {
        self.items.pop()
    }
}

pub struct SortedVecOfArcSlice<'s, TKey: Ord, TValue: EntityWithKey<TKey> + Clone> {
    slice: &'s [Arc<TValue>],
    itm: std::marker::PhantomData<TKey>,
}

impl<'s, TKey: Ord, TValue: EntityWithKey<TKey> + Clone> std::ops::Deref
    for SortedVecOfArcSlice<'s, TKey, TValue>
{
    type Target = [Arc<TValue>];

    fn deref(&self) -> &Self::Target {
        self.slice
    }
}

impl<'s, TKey: Ord, TValue: EntityWithKey<TKey> + Clone> AsRef<[Arc<TValue>]>
    for SortedVecOfArcSlice<'s, TKey, TValue>
{
    fn as_ref(&self) -> &[Arc<TValue>] {
        self.slice
    }
}

impl<'s, TKey: Ord, TValue: EntityWithKey<TKey> + Clone> SortedVecOfArcSlice<'s, TKey, TValue> {
    fn new(slice: &'s [Arc<TValue>]) -> Self {
        Self {
            slice,
            itm: std::marker::PhantomData,
        }
    }

    pub fn to_sorted_vec(self) -> SortedVecOfArc<TKey, TValue> {
        SortedVecOfArc {
            items: self.slice.to_vec(),
            itm: self.itm,
        }
    }

    pub fn to_vec(self) -> Vec<Arc<TValue>> {
        self.slice.to_vec()
    }

    pub fn len(&self) -> usize {
        self.slice.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slice.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'s, Arc<TValue>> {
        self.slice.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestEntity {
        key: u8,
        value: u8,
    }

    impl EntityWithKey<u8> for TestEntity {
        fn get_key(&self) -> &u8 {
            &self.key
        }
    }

    #[test]
    fn test_truncate_capacity() {
        let mut vec = SortedVecOfArc::new();

        vec.insert_or_replace(Arc::new(TestEntity { key: 2, value: 2 }));
        vec.insert_or_replace(Arc::new(TestEntity { key: 1, value: 1 }));
        vec.insert_or_replace(Arc::new(TestEntity { key: 3, value: 3 }));

        vec.truncate_capacity(2);

        let values = vec
            .as_slice()
            .iter()
            .map(|itm| itm.value)
            .collect::<Vec<_>>();
        assert_eq!(vec![1, 2], values);
    }
}
