use std::mem;
use std::sync::Arc;

use crate::sorted_vec::EntityWithStrKey;

use super::*;

#[derive(Clone, Debug)]
pub struct SortedVecOfArcWith2StrKey<TValue: super::EntityWith2StrKey> {
    pub(crate) partitions: Vec<super::PartitionOfArc<TValue>>,
    len: usize,
}

impl<TValue: super::EntityWith2StrKey> Default for SortedVecOfArcWith2StrKey<TValue> {
    fn default() -> Self {
        Self::new()
    }
}

impl<TValue: super::EntityWith2StrKey> SortedVecOfArcWith2StrKey<TValue> {
    pub fn new() -> Self {
        Self {
            partitions: Vec::new(),
            len: 0,
        }
    }

    pub fn drain_into_vec(&mut self) -> Vec<Arc<TValue>> {
        let len = self.len;
        let mut items_to_drain = Vec::new();

        mem::swap(&mut self.partitions, &mut items_to_drain);
        self.len = 0;

        let mut result = Vec::with_capacity(len);
        for sub_items in items_to_drain {
            result.extend(sub_items.into_vec());
        }
        result
    }

    pub(crate) fn insert_item_to_new_partition(
        &mut self,
        index: usize,
        items: PartitionOfArc<TValue>,
    ) {
        self.partitions.insert(index, items);
        self.len += 1;
    }

    pub(crate) fn insert_item_to_existing_partition(
        &mut self,
        partition_index: usize,
        row_index: usize,
        item: Arc<TValue>,
    ) {
        let sub_items = self.partitions.get_mut(partition_index);
        if let Some(sub_items) = sub_items {
            sub_items.rows.insert(row_index, item);
            self.len += 1;
        }
    }

    // Returns the index of the inserted item and old item if it was replaced
    pub fn insert_or_replace(&mut self, item: Arc<TValue>) -> Option<Arc<TValue>> {
        match self.get_index(item.get_primary_key()) {
            Ok(partition_index) => {
                let partition = self.partitions.get_mut(partition_index).unwrap();
                let (_, result) = partition.insert_or_replace(item);
                result
            }
            Err(index_to_insert) => {
                let sub_items = super::PartitionOfArc::new(item);
                self.partitions.insert(index_to_insert, sub_items);
                self.len += 1;
                None
            }
        }
    }

    pub fn insert_or_if_not_exists<'s>(
        &'s mut self,
        primary_key: &str,
        secondary_key: &str,
    ) -> InsertIfNotExists2KeysArc<'s, TValue> {
        match self.get_index(primary_key) {
            Ok(partition_index) => {
                let partition = self.partitions.get_mut(partition_index).unwrap();
                match partition.binary_search(secondary_key) {
                    Ok(_) => InsertIfNotExists2KeysArc::Exists,
                    Err(row_index) => InsertIfNotExists2KeysArc::Insert(InsertEntity2KeysArc::new(
                        partition_index,
                        Some(row_index),
                        self,
                    )),
                }
            }
            Err(partition_index) => InsertIfNotExists2KeysArc::Insert(InsertEntity2KeysArc::new(
                partition_index,
                None,
                self,
            )),
        }
    }

    fn get_index(&self, primary_key: &str) -> Result<usize, usize> {
        self.partitions
            .binary_search_by(|itm| itm.get_key().cmp(primary_key))
    }

    pub fn contains(&self, primary_key: &str, secondary_key: &str) -> bool {
        match self.get_index(primary_key) {
            Ok(partition_index) => {
                let partition = self.partitions.get(partition_index).unwrap();
                partition.contains(secondary_key)
            }
            Err(_) => false,
        }
    }

    pub fn get(&self, primary_key: &str, secondary_key: &str) -> Option<&Arc<TValue>> {
        match self.get_index(primary_key) {
            Ok(partition_index) => {
                let partition = self.partitions.get(partition_index).unwrap();
                partition.get(secondary_key)
            }
            Err(_) => None,
        }
    }

    pub fn get_by_primary_key<'s>(
        &'s self,
        primary_key: &str,
    ) -> Option<std::slice::Iter<'s, Arc<TValue>>> {
        match self.get_index(primary_key) {
            Ok(partition_index) => {
                let partition = self.partitions.get(partition_index).unwrap();
                Some(partition.iter())
            }
            Err(_) => None,
        }
    }

    pub fn get_by_primary_key_mut<'s>(
        &'s mut self,
        primary_key: &str,
    ) -> Option<std::slice::IterMut<'s, Arc<TValue>>> {
        match self.get_index(primary_key) {
            Ok(partition_index) => {
                let partition = self.partitions.get_mut(partition_index).unwrap();
                Some(partition.iter_mut())
            }
            Err(_) => None,
        }
    }

    pub fn get_mut_or_create<'s>(
        &'s mut self,
        primary_key: &str,
        secondary_key: &str,
    ) -> GetMutOrCreateEntry2KeysArc<'s, TValue> {
        match self.get_index(primary_key) {
            Ok(partition_index) => {
                let row_index = {
                    let partition = self.partitions.get_mut(partition_index).unwrap();
                    partition.get_index(secondary_key)
                };

                match row_index {
                    Ok(row_index) => {
                        return GetMutOrCreateEntry2KeysArc::GetMut(
                            self.partitions
                                .get_mut(partition_index)
                                .unwrap()
                                .rows
                                .get_mut(row_index)
                                .unwrap(),
                        );
                    }
                    Err(row_index) => GetMutOrCreateEntry2KeysArc::Create(
                        InsertEntity2KeysArc::new(partition_index, Some(row_index), self),
                    ),
                }
            }
            Err(partition_index) => GetMutOrCreateEntry2KeysArc::Create(InsertEntity2KeysArc::new(
                partition_index,
                None,
                self,
            )),
        }
    }

    pub fn get_mut(&mut self, primary_key: &str, secondary_key: &str) -> Option<&mut Arc<TValue>> {
        match self.get_index(primary_key) {
            Ok(partition_index) => {
                let partition = self.partitions.get_mut(partition_index).unwrap();
                partition.get_mut(secondary_key)
            }
            Err(_) => None,
        }
    }

    pub fn get_from_key_to_up(
        &self,
        primary_key: &str,
        secondary_key: &str,
    ) -> Option<&[Arc<TValue>]> {
        match self.get_index(primary_key) {
            Ok(partition_index) => {
                let partition = self.partitions.get(partition_index).unwrap();
                Some(partition.get_from_key_to_up(secondary_key))
            }
            Err(_) => None,
        }
    }

    pub fn get_from_bottom_to_key(
        &self,
        primary_key: &str,
        secondary_key: &str,
    ) -> Option<&[Arc<TValue>]> {
        match self.get_index(primary_key) {
            Ok(partition_index) => {
                let partition = self.partitions.get(partition_index).unwrap();
                Some(partition.get_from_bottom_to_key(secondary_key))
            }
            Err(_) => None,
        }
    }

    pub fn remove(&mut self, primary_key: &str, secondary_key: &str) -> Option<Arc<TValue>> {
        match self.get_index(primary_key) {
            Ok(partition_index) => {
                let partition = self.partitions.get_mut(partition_index).unwrap();
                let removed_item = partition.remove(secondary_key);

                if removed_item.is_some() {
                    self.len -= 1;
                }

                removed_item
            }
            Err(_) => None,
        }
    }

    pub fn clear(&mut self) {
        self.partitions.clear();
    }

    pub fn first(&self) -> Option<&Arc<TValue>> {
        let partition = self.partitions.first()?;
        partition.first()
    }

    pub fn first_mut(&mut self) -> Option<&mut Arc<TValue>> {
        let partition = self.partitions.first_mut()?;
        partition.first_mut()
    }

    pub fn last(&self) -> Option<&Arc<TValue>> {
        let partition = self.partitions.last()?;
        partition.last()
    }

    pub fn last_mut(&mut self) -> Option<&mut Arc<TValue>> {
        let partition = self.partitions.last_mut()?;
        partition.last_mut()
    }

    pub fn into_vec(self) -> Vec<Arc<TValue>> {
        let mut result = Vec::with_capacity(self.len);
        for sub_items in self.partitions {
            result.extend(sub_items.into_vec());
        }
        result
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn partitions_len(&self) -> usize {
        self.partitions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn range(&self, primary_key: &str, range: std::ops::Range<&str>) -> Option<&[Arc<TValue>]> {
        match self.get_index(primary_key) {
            Ok(partition_index) => {
                let partition = self.partitions.get(partition_index).unwrap();
                Some(partition.range(range))
            }
            Err(_) => None,
        }
    }

    pub fn remove_by_primary_key(&mut self, primary_key: &str) -> Option<Vec<Arc<TValue>>> {
        match self.get_index(primary_key) {
            Ok(index) => {
                let partition = self.partitions.remove(index);
                self.len -= partition.len();
                Some(partition.into_vec())
            }
            Err(_) => None,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<TValue>> {
        self.partitions
            .iter()
            .flat_map(|partition| partition.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Arc<TValue>> {
        self.partitions
            .iter_mut()
            .flat_map(|partition| partition.iter_mut())
    }

    pub fn get_len_and_capacity(&self) -> (usize, usize) {
        let mut len = 0;
        let mut capacity = 0;

        for itm in self.partitions.iter() {
            len += itm.len();
            capacity += itm.get_capacity();
        }

        (len, capacity)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[derive(Debug, Clone)]
    pub struct TestEntity {
        pub primary_key: String,
        pub secondary_key: String,
        pub value: u8,
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
    fn test_insert_or_replace() {
        let mut items = SortedVecOfArcWith2StrKey::new();

        let item_to_insert = Arc::new(TestEntity {
            primary_key: "pk".to_string(),
            secondary_key: "key1".to_string(),
            value: 1,
        });

        match items.insert_or_if_not_exists(
            item_to_insert.get_primary_key(),
            item_to_insert.get_secondary_key(),
        ) {
            InsertIfNotExists2KeysArc::Insert(insert_entity) => {
                insert_entity.insert(item_to_insert);
            }
            InsertIfNotExists2KeysArc::Exists => {
                panic!("Should not be here")
            }
        }

        assert_eq!(items.len(), 1);
        assert_eq!(items.partitions_len(), 1);

        let item_to_insert = Arc::new(TestEntity {
            primary_key: "pk".to_string(),
            secondary_key: "key2".to_string(),
            value: 2,
        });

        match items.insert_or_if_not_exists(
            item_to_insert.get_primary_key(),
            item_to_insert.get_secondary_key(),
        ) {
            InsertIfNotExists2KeysArc::Insert(insert_entity) => {
                insert_entity.insert(item_to_insert);
            }
            InsertIfNotExists2KeysArc::Exists => {
                panic!("Should not be here")
            }
        }

        assert_eq!(items.len(), 2);
        assert_eq!(items.partitions_len(), 1);

        match items.insert_or_if_not_exists("pk", "key2") {
            InsertIfNotExists2KeysArc::Insert(_) => {
                panic!("Should not be here")
            }
            InsertIfNotExists2KeysArc::Exists => {}
        }

        let item = items.get("pk", "key2").unwrap();

        assert_eq!(item.value, 2)
    }
}
