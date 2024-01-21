use crate::sorted_vec::{
    EntityWithStrKey, GetMutOrCreateEntry, InsertEntity, InsertOrUpdateEntry, UpdateEntry,
};

use super::InsertIfNotExists;

#[derive(Clone)]
pub struct SortedVecWithStrKey<TValue: EntityWithStrKey + Clone> {
    items: Vec<TValue>,
}

impl<TValue: EntityWithStrKey + Clone> SortedVecWithStrKey<TValue> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn new_with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
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

    pub fn insert_or_if_not_exists<'s>(&'s mut self, key: &str) -> InsertIfNotExists<'s, TValue> {
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

    pub fn insert_or_update<'s>(&'s mut self, key: &str) -> InsertOrUpdateEntry<'s, TValue> {
        let insert_index = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match insert_index {
            Ok(index) => {
                InsertOrUpdateEntry::Update(UpdateEntry::new(index, &mut self.items[index]))
            }
            Err(index) => InsertOrUpdateEntry::Insert(InsertEntity::new(index, &mut self.items)),
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));
        result.is_ok()
    }

    pub fn get(&self, key: &str) -> Option<&TValue> {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => self.items.get(index),
            Err(_) => None,
        }
    }

    pub fn get_mut_or_create<'s>(&'s mut self, key: &str) -> GetMutOrCreateEntry<'s, TValue> {
        let index = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match index {
            Ok(index) => GetMutOrCreateEntry::GetMut(&mut self.items[index]),
            Err(index) => GetMutOrCreateEntry::Create(InsertEntity::new(index, &mut self.items)),
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut TValue> {
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

    pub fn get_from_key_to_up(&self, key: &str) -> &[TValue] {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => &self.items[index..],
            Err(index) => &self.items[index..],
        }
    }

    pub fn get_from_bottom_to_key(&self, key: &str) -> &[TValue] {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => &self.items[..=index],
            Err(index) => &self.items[..=index],
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<TValue> {
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

    pub fn iter(&self) -> std::slice::Iter<TValue> {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<TValue> {
        self.items.iter_mut()
    }

    pub fn into_vec(self) -> Vec<TValue> {
        self.items
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

    use crate::sorted_vec::EntityWithStrKey;

    #[derive(Debug, Clone)]
    pub struct TestEntity {
        pub key: String,
        pub value: u8,
    }

    impl EntityWithStrKey for TestEntity {
        fn get_key(&self) -> &str {
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
        let mut vec = super::SortedVecWithStrKey::new();

        vec.insert_or_replace(TestEntity {
            key: "5".to_string(),
            value: 5,
        });

        vec.insert_or_replace(TestEntity {
            key: "4".to_string(),
            value: 4,
        });

        vec.insert_or_replace(TestEntity {
            key: "9".to_string(),
            value: 9,
        });

        assert_eq!(
            vec!["4", "5", "9"],
            vec.items.iter().map(|x| x.key.as_str()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn debugging_insert_or_update() {
        let mut vec = super::SortedVecWithStrKey::new();

        vec.insert_or_replace(TestEntity {
            key: "5".to_string(),
            value: 5,
        });

        let result = match vec.insert_or_update("5") {
            crate::sorted_vec::InsertOrUpdateEntry::Insert(insert) => {
                insert.insert_and_get_index(TestEntity {
                    key: "5".to_string(),
                    value: 5,
                })
            }
            crate::sorted_vec::InsertOrUpdateEntry::Update(update) => {
                update.item.value = 7;
                update.index
            }
        };

        assert_eq!(0, result);

        println!("result: {:?}", vec.items);

        let result = match vec.insert_or_update("6") {
            crate::sorted_vec::InsertOrUpdateEntry::Insert(insert) => {
                insert.insert_and_get_index(TestEntity {
                    key: "6".to_string(),
                    value: 6,
                })
            }
            crate::sorted_vec::InsertOrUpdateEntry::Update(update) => {
                update.item.value = 9;
                update.index
            }
        };

        assert_eq!(1, result);

        println!("result: {:?}", vec.items);
    }
}
