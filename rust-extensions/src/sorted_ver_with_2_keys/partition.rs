use crate::sorted_vec::EntityWithStrKey;

use super::*;
use crate::sorted_vec::*;

#[derive(Debug, Clone)]
pub struct Partition<TValue: EntityWith2StrKey> {
    pub(crate) rows: Vec<TValue>,
}

impl<TValue: EntityWith2StrKey> EntityWithStrKey for Partition<TValue> {
    fn get_key(&self) -> &str {
        self.rows.get(0).unwrap().get_primary_key()
    }
}

impl<TValue: EntityWith2StrKey> Partition<TValue> {
    pub fn new(item: TValue) -> Self {
        Self { rows: vec![item] }
    }

    pub fn insert_or_replace(&mut self, item: TValue) -> (usize, Option<TValue>) {
        let insert_index = self
            .rows
            .binary_search_by(|itm| itm.get_secondary_key().cmp(item.get_secondary_key()));

        match insert_index {
            Ok(index) => {
                let got = std::mem::replace(&mut self.rows[index], item);
                (index, Some(got))
            }
            Err(index) => {
                self.rows.insert(index, item);
                (index, None)
            }
        }
    }

    pub fn binary_search(&mut self, secondary_key: &str) -> Result<usize, usize> {
        self.rows
            .binary_search_by(|itm| itm.get_secondary_key().cmp(secondary_key))
    }

    pub fn insert_or_update<'s>(&'s mut self, key: &str) -> InsertOrUpdateEntry<'s, TValue> {
        let insert_index = self
            .rows
            .binary_search_by(|itm| itm.get_secondary_key().cmp(key));

        match insert_index {
            Ok(index) => {
                InsertOrUpdateEntry::Update(UpdateEntry::new(index, &mut self.rows[index]))
            }
            Err(index) => InsertOrUpdateEntry::Insert(InsertEntity::new(index, &mut self.rows)),
        }
    }

    pub(crate) fn get_index(&self, secondary_key: &str) -> Result<usize, usize> {
        self.rows
            .binary_search_by(|itm| itm.get_secondary_key().cmp(secondary_key))
    }

    pub fn contains(&self, key: &str) -> bool {
        let result = self
            .rows
            .binary_search_by(|itm| itm.get_secondary_key().cmp(key));
        result.is_ok()
    }

    pub fn get(&self, key: &str) -> Option<&TValue> {
        let result = self
            .rows
            .binary_search_by(|itm| itm.get_secondary_key().cmp(key));

        match result {
            Ok(index) => self.rows.get(index),
            Err(_) => None,
        }
    }

    pub fn get_mut_or_create<'s>(&'s mut self, key: &str) -> GetMutOrCreateEntry<'s, TValue> {
        let index = self
            .rows
            .binary_search_by(|itm| itm.get_secondary_key().cmp(key));

        match index {
            Ok(index) => GetMutOrCreateEntry::GetMut(&mut self.rows[index]),
            Err(index) => GetMutOrCreateEntry::Create(InsertEntity::new(index, &mut self.rows)),
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut TValue> {
        let result = self
            .rows
            .binary_search_by(|itm| itm.get_secondary_key().cmp(key));

        match result {
            Ok(index) => self.rows.get_mut(index),
            Err(_) => None,
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<TValue> {
        let result = self
            .rows
            .binary_search_by(|itm| itm.get_secondary_key().cmp(key));

        match result {
            Ok(index) => Some(self.rows.remove(index)),
            Err(_) => None,
        }
    }

    pub fn get_from_key_to_up(&self, secondary_key: &str) -> &[TValue] {
        let result = self
            .rows
            .binary_search_by(|itm| itm.get_secondary_key().cmp(secondary_key));

        match result {
            Ok(index) => &self.rows[index..],
            Err(index) => &self.rows[index..],
        }
    }

    pub fn get_from_bottom_to_key(&self, secondary_key: &str) -> &[TValue] {
        if self.rows.is_empty() {
            return &self.rows[0..0];
        }

        let result = self
            .rows
            .binary_search_by(|itm| itm.get_secondary_key().cmp(secondary_key));

        match result {
            Ok(index) => &self.rows[..=index],
            Err(index) => &self.rows[..index],
        }
    }

    pub fn clear(&mut self) {
        self.rows.clear();
    }

    pub fn truncate_capacity(&mut self, capacity: usize) {
        self.rows.truncate(capacity);
    }

    pub fn first(&self) -> Option<&TValue> {
        self.rows.first()
    }

    pub fn first_mut(&mut self) -> Option<&mut TValue> {
        self.rows.first_mut()
    }

    pub fn last(&self) -> Option<&TValue> {
        self.rows.last()
    }

    pub fn last_mut(&mut self) -> Option<&mut TValue> {
        self.rows.last_mut()
    }

    pub fn iter<'s>(&'s self) -> std::slice::Iter<'s, TValue> {
        self.rows.iter()
    }

    pub fn iter_mut<'s>(&'s mut self) -> std::slice::IterMut<'s, TValue> {
        self.rows.iter_mut()
    }

    pub fn into_vec(self) -> Vec<TValue> {
        self.rows
    }

    pub fn as_slice(&self) -> &[TValue] {
        self.rows.as_slice()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn range(&self, range: std::ops::Range<&str>) -> &[TValue] {
        let index_from = self
            .rows
            .binary_search_by(|itm| itm.get_secondary_key().cmp(&range.start));

        let index_from = match index_from {
            Ok(index) => index,
            Err(index) => index,
        };

        let index_to = self
            .rows
            .binary_search_by(|itm| itm.get_secondary_key().cmp(&range.end));

        match index_to {
            Ok(index_to) => {
                return &self.rows[index_from..=index_to];
            }
            Err(index_to) => &self.rows[index_from..index_to],
        }
    }
    pub fn get_capacity(&self) -> usize {
        self.rows.capacity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestEntity {
        primary_key: String,
        secondary_key: String,
        value: u8,
    }

    impl EntityWith2StrKey for TestEntity {
        fn get_primary_key(&self) -> &str {
            &self.primary_key
        }

        fn get_secondary_key(&self) -> &str {
            &self.secondary_key
        }
    }

    #[test]
    fn test_truncate_capacity() {
        let mut partition = Partition::new(TestEntity {
            primary_key: "p".to_string(),
            secondary_key: "b".to_string(),
            value: 2,
        });

        partition.insert_or_replace(TestEntity {
            primary_key: "p".to_string(),
            secondary_key: "a".to_string(),
            value: 1,
        });
        partition.insert_or_replace(TestEntity {
            primary_key: "p".to_string(),
            secondary_key: "c".to_string(),
            value: 3,
        });

        partition.truncate_capacity(2);

        let values = partition
            .rows
            .iter()
            .map(|itm| itm.value)
            .collect::<Vec<_>>();
        assert_eq!(vec![1, 2], values);
    }
}
