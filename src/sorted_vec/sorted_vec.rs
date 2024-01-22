use crate::sorted_vec::{
    EntityWithKey, GetMutOrCreateEntry, InsertEntity, InsertOrUpdateEntry, UpdateEntry,
};

use super::InsertIfNotExists;

pub struct SortedVec<TKey: Ord, TValue: EntityWithKey<TKey> + Clone> {
    items: Vec<TValue>,
    itm: std::marker::PhantomData<TKey>,
}

impl<TKey: Ord, TValue: EntityWithKey<TKey> + Clone> SortedVec<TKey, TValue> {
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

    pub fn get_from_key_to_up(&self, key: &TKey) -> &[TValue] {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => &self.items[index..],
            Err(index) => &self.items[index..],
        }
    }

    pub fn get_from_bottom_to_key(&self, key: &TKey) -> &[TValue] {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => &self.items[..=index],
            Err(index) => &self.items[..=index],
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

    pub fn iter(&self) -> std::slice::Iter<TValue> {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<TValue> {
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
}
