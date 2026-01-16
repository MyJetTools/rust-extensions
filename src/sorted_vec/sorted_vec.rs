use crate::sorted_vec::{
    EntityWithKey, GetMutOrCreateEntry, InsertEntity, InsertOrUpdateEntry, UpdateEntry,
};

use super::InsertIfNotExists;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct SortedVec<TKey: Ord, TValue: EntityWithKey<TKey>> {
    items: Vec<TValue>,
    itm: std::marker::PhantomData<TKey>,
}

impl<TKey: Ord, TValue: EntityWithKey<TKey>> SortedVec<TKey, TValue> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
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

    pub fn new_with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            itm: std::marker::PhantomData,
        }
    }

    // Returns the index of the inserted item and old item if it was replaced
    pub fn insert_or_replace(&mut self, item: TValue) -> (usize, Option<TValue>) {
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

    pub fn insert_or_if_not_exists<'s>(&'s mut self, key: &TKey) -> InsertIfNotExists<'s, TValue> {
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

    pub fn get_mut_or_create<'s>(&'s mut self, key: &TKey) -> GetMutOrCreateEntry<'s, TValue> {
        let index = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match index {
            Ok(index) => GetMutOrCreateEntry::GetMut(&mut self.items[index]),
            Err(index) => GetMutOrCreateEntry::Create(InsertEntity::new(index, &mut self.items)),
        }
    }

    pub fn insert_or_update<'s>(&'s mut self, key: &TKey) -> InsertOrUpdateEntry<'s, TValue> {
        let insert_index = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match insert_index {
            Ok(index) => {
                InsertOrUpdateEntry::Update(UpdateEntry::new(index, &mut self.items[index]))
            }
            Err(index) => InsertOrUpdateEntry::Insert(InsertEntity::new(index, &mut self.items)),
        }
    }

    pub fn contains(&self, key: &TKey) -> bool {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));
        result.is_ok()
    }

    pub fn get(&self, key: &TKey) -> Option<&TValue> {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => self.items.get(index),
            Err(_) => None,
        }
    }

    pub fn get_mut(&mut self, key: &TKey) -> Option<&mut TValue> {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => self.items.get_mut(index),
            Err(_) => None,
        }
    }

    pub fn get_by_index(&self, index: usize) -> Option<&TValue> {
        self.items.get(index)
    }

    pub fn get_by_index_mut(&mut self, index: usize) -> Option<&mut TValue> {
        self.items.get_mut(index)
    }

    pub fn range(&self, range: std::ops::Range<TKey>) -> &[TValue] {
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

    pub fn range_by_index<R: std::ops::RangeBounds<usize>>(&self, range: R) -> &[TValue] {
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
            return &self.items[0..0];
        }

        &self.items[start..end]
    }

    pub fn get_from_key_to_up(&self, key: &TKey) -> &[TValue] {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => &self.items[index..],
            Err(index) => &self.items[index..],
        }
    }

    pub fn get_from_bottom_to_key(&self, key: &TKey) -> &[TValue] {
        if self.items.is_empty() {
            return &self.items[0..0];
        }

        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => &self.items[..=index],
            Err(index) => &self.items[..index],
        }
    }

    pub fn remove(&mut self, key: &TKey) -> Option<TValue> {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => Some(self.items.remove(index)),
            Err(_) => None,
        }
    }

    pub fn remove_at(&mut self, index: usize) -> Option<TValue> {
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

    pub fn truncate_capacity(&mut self, capacity: usize) {
        self.items.truncate(capacity);
    }

    pub fn first(&self) -> Option<&TValue> {
        self.items.first()
    }

    pub fn first_mut(&mut self) -> Option<&mut TValue> {
        self.items.first_mut()
    }

    pub fn last(&self) -> Option<&TValue> {
        self.items.last()
    }

    pub fn last_mut(&mut self) -> Option<&mut TValue> {
        self.items.last_mut()
    }

    pub fn into_vec(self) -> Vec<TValue> {
        self.items
    }

    pub fn iter<'s>(&'s self) -> std::slice::Iter<'s, TValue> {
        self.items.iter()
    }

    pub fn iter_mut<'s>(&'s mut self) -> std::slice::IterMut<'s, TValue> {
        self.items.iter_mut()
    }

    pub fn as_slice(&self) -> &[TValue] {
        self.items.as_slice()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get_highest_and_below_amount(&self, highest_key: &TKey, amount: usize) -> &[TValue] {
        if self.items.is_empty() {
            return &self.items[0..0];
        }

        let index_to = self
            .items
            .binary_search_by(|itm| itm.get_key().cmp(highest_key));

        let mut index_to = match index_to {
            Ok(index_to) => index_to,
            Err(index_to) => index_to,
        };

        if index_to >= self.items.len() {
            index_to = self.items.len() - 1;
        }

        let index_to_key = self.items[index_to].get_key();
        if index_to_key <= highest_key {
            if amount >= index_to {
                return &self.items[..=index_to];
            }

            return &self.items[index_to - amount + 1..=index_to];
        }

        if amount >= index_to {
            return &self.items[..index_to];
        }

        &self.items[index_to - amount + 1..index_to]
    }

    pub fn drain_into_vec(&mut self) -> Vec<TValue> {
        let mut result = Vec::with_capacity(self.items.len());
        while let Some(item) = self.items.pop() {
            result.push(item);
        }
        result
    }

    pub fn pop(&mut self) -> Option<TValue> {
        self.items.pop()
    }
}

#[cfg(test)]
mod tests {

    use crate::sorted_vec::EntityWithKey;

    #[derive(Debug, Clone)]
    pub struct TestEntity {
        pub key: u8,
        pub value: u8,
    }

    impl EntityWithKey<u8> for TestEntity {
        fn get_key(&self) -> &u8 {
            &self.key
        }
    }

    #[test]
    fn test_cmp() {
        let src = vec![1, 2, 3, 5, 6];

        let result = src.binary_search(&4);

        println!("result: {:?}", result);
    }

    #[test]
    fn test_basic_inserts() {
        let mut vec = super::SortedVec::new();

        vec.insert_or_replace(TestEntity { key: 5, value: 5 });

        vec.insert_or_replace(TestEntity { key: 4, value: 6 });

        vec.insert_or_replace(TestEntity { key: 9, value: 9 });

        assert_eq!(
            vec![4u8, 5u8, 9u8],
            vec.items.iter().map(|x| x.key).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_insert_third_is_first() {
        let mut vec = super::SortedVec::new();

        vec.insert_or_replace(TestEntity { key: 5, value: 5 });

        vec.insert_or_replace(TestEntity { key: 4, value: 4 });

        vec.insert_or_replace(TestEntity { key: 3, value: 3 });

        vec.insert_or_replace(TestEntity { key: 1, value: 1 });

        vec.insert_or_replace(TestEntity { key: 2, value: 2 });

        assert_eq!(
            vec![1u8, 2u8, 3u8, 4u8, 5u8],
            vec.items.iter().map(|x| x.key).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_range_at_exact_items() {
        let mut vec = super::SortedVec::new();

        vec.insert_or_replace(TestEntity { key: 1, value: 1 });

        vec.insert_or_replace(TestEntity { key: 2, value: 2 });

        vec.insert_or_replace(TestEntity { key: 3, value: 3 });

        vec.insert_or_replace(TestEntity { key: 4, value: 4 });

        vec.insert_or_replace(TestEntity { key: 5, value: 5 });

        let result = vec.range(2..4);

        assert_eq!(3, result.len());
        assert_eq!(
            vec![2u8, 3u8, 4u8],
            result.into_iter().map(|itm| itm.value).collect::<Vec<u8>>()
        );
    }

    #[test]
    fn test_range_at_not_exact() {
        let mut vec = super::SortedVec::new();

        vec.insert_or_replace(TestEntity { key: 1, value: 1 });

        vec.insert_or_replace(TestEntity { key: 3, value: 3 });

        vec.insert_or_replace(TestEntity { key: 4, value: 4 });

        vec.insert_or_replace(TestEntity { key: 6, value: 6 });

        vec.insert_or_replace(TestEntity { key: 7, value: 7 });

        let result = vec.range(2..5);

        assert_eq!(2, result.len());
        assert_eq!(
            vec![3u8, 4u8],
            result.into_iter().map(|itm| itm.value).collect::<Vec<u8>>()
        );

        let result = vec.range(1..10);

        assert_eq!(5, result.len());
        assert_eq!(
            vec![1u8, 3u8, 4u8, 6u8, 7u8],
            result.into_iter().map(|itm| itm.value).collect::<Vec<u8>>()
        );

        let result = vec.range(2..7);

        assert_eq!(4, result.len());
        assert_eq!(
            vec![3u8, 4u8, 6u8, 7u8],
            result.into_iter().map(|itm| itm.value).collect::<Vec<u8>>()
        );
    }

    #[test]
    fn test_sub_sequence() {
        let mut vec = super::SortedVec::new();

        vec.insert_or_replace(TestEntity { key: 1, value: 1 });
        vec.insert_or_replace(TestEntity { key: 3, value: 3 });
        vec.insert_or_replace(TestEntity { key: 4, value: 4 });
        vec.insert_or_replace(TestEntity { key: 6, value: 6 });
        vec.insert_or_replace(TestEntity { key: 7, value: 7 });

        let sub = vec.sub_sequence(2..7);

        assert_eq!(4, sub.len());
        assert_eq!(
            vec![3u8, 4u8, 6u8, 7u8],
            sub.iter().map(|itm| itm.value).collect::<Vec<u8>>()
        );
    }

    #[test]
    fn test_range_by_index() {
        let mut vec = super::SortedVec::new();

        vec.insert_or_replace(TestEntity { key: 1, value: 1 });
        vec.insert_or_replace(TestEntity { key: 3, value: 3 });
        vec.insert_or_replace(TestEntity { key: 4, value: 4 });
        vec.insert_or_replace(TestEntity { key: 6, value: 6 });
        vec.insert_or_replace(TestEntity { key: 7, value: 7 });

        let all = vec.range_by_index(..);
        assert_eq!(
            vec![1u8, 3u8, 4u8, 6u8, 7u8],
            all.iter().map(|itm| itm.value).collect::<Vec<u8>>()
        );

        let from = vec.range_by_index(2..);
        assert_eq!(
            vec![4u8, 6u8, 7u8],
            from.iter().map(|itm| itm.value).collect::<Vec<u8>>()
        );

        let to = vec.range_by_index(..2);
        assert_eq!(
            vec![1u8, 3u8],
            to.iter().map(|itm| itm.value).collect::<Vec<u8>>()
        );

        let mid = vec.range_by_index(1..4);
        assert_eq!(
            vec![3u8, 4u8, 6u8],
            mid.iter().map(|itm| itm.value).collect::<Vec<u8>>()
        );

        let inclusive = vec.range_by_index(..=0);
        assert_eq!(
            vec![1u8],
            inclusive.iter().map(|itm| itm.value).collect::<Vec<u8>>()
        );

        let empty = vec.range_by_index(10..);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_get_highest_and_amount() {
        let mut vec = super::SortedVec::new();

        vec.insert_or_replace(TestEntity { key: 1, value: 1 });

        vec.insert_or_replace(TestEntity { key: 3, value: 3 });

        vec.insert_or_replace(TestEntity { key: 4, value: 4 });

        vec.insert_or_replace(TestEntity { key: 6, value: 6 });

        vec.insert_or_replace(TestEntity { key: 7, value: 7 });

        let result = vec.get_highest_and_below_amount(&6, 2);

        assert_eq!(
            vec![4u8, 6u8],
            result.into_iter().map(|itm| itm.value).collect::<Vec<u8>>()
        );

        let result = vec.get_highest_and_below_amount(&7, 2);

        assert_eq!(
            vec![6u8, 7u8],
            result.into_iter().map(|itm| itm.value).collect::<Vec<u8>>()
        );

        let result = vec.get_highest_and_below_amount(&7, 3);

        assert_eq!(
            vec![4u8, 6u8, 7u8],
            result.into_iter().map(|itm| itm.value).collect::<Vec<u8>>()
        );

        let result = vec.get_highest_and_below_amount(&6, 5);

        assert_eq!(
            vec![1u8, 3u8, 4u8, 6u8],
            result.into_iter().map(|itm| itm.value).collect::<Vec<u8>>()
        );

        let result = vec.get_highest_and_below_amount(&8, 5);

        assert_eq!(
            vec![1u8, 3u8, 4u8, 6u8, 7u8],
            result.into_iter().map(|itm| itm.value).collect::<Vec<u8>>()
        );

        let result = vec.get_highest_and_below_amount(&8, 6);

        assert_eq!(
            vec![1u8, 3u8, 4u8, 6u8, 7u8],
            result.into_iter().map(|itm| itm.value).collect::<Vec<u8>>()
        );

        let result = vec.get_highest_and_below_amount(&0, 5);

        assert_eq!(
            Vec::<u8>::new(),
            result.into_iter().map(|itm| itm.value).collect::<Vec<u8>>()
        );
    }

    #[test]
    fn test_get_from_bottom_to_key_bounds() {
        let mut vec = super::SortedVec::new();

        let result = vec.get_from_bottom_to_key(&1);
        assert_eq!(0, result.len());

        vec.insert_or_replace(TestEntity { key: 2, value: 2 });
        vec.insert_or_replace(TestEntity { key: 4, value: 4 });

        let below_first = vec.get_from_bottom_to_key(&1);
        assert_eq!(0, below_first.len());

        let between = vec.get_from_bottom_to_key(&3);
        assert_eq!(1, between.len());
        assert_eq!(2, between[0].value);

        let above_last = vec.get_from_bottom_to_key(&5);
        assert_eq!(
            vec![2u8, 4u8],
            above_last.iter().map(|itm| itm.value).collect::<Vec<u8>>()
        );
    }

    #[test]
    fn test_get_highest_and_below_amount_empty() {
        let vec = super::SortedVec::<u8, TestEntity>::new();

        let result = vec.get_highest_and_below_amount(&5, 3);
        assert_eq!(0, result.len());
    }
}
