use std::mem;

use crate::sorted_vec::*;

use super::*;

#[derive(Clone, Debug)]
pub struct SortedVecWith2StrKey<TValue: super::EntityWith2StrKey> {
    pub(crate) partitions: Vec<super::Partition<TValue>>,
    len: usize,
}

impl<TValue: super::EntityWith2StrKey> SortedVecWith2StrKey<TValue> {
    pub fn new() -> Self {
        Self {
            partitions: Vec::new(),
            len: 0,
        }
    }

    pub fn drain_into_vec(&mut self) -> Vec<TValue> {
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

    pub(crate) fn insert_item_to_new_partition(&mut self, index: usize, items: Partition<TValue>) {
        self.partitions.insert(index, items);
        self.len += 1;
    }

    pub(crate) fn insert_item_to_existing_partition(
        &mut self,
        partition_index: usize,
        row_index: usize,
        item: TValue,
    ) {
        let sub_items = self.partitions.get_mut(partition_index);
        if let Some(sub_items) = sub_items {
            sub_items.rows.insert(row_index, item);
            self.len += 1;
        }
    }

    // Returns the index of the inserted item and old item if it was replaced
    pub fn insert_or_replace(&mut self, item: TValue) -> Option<TValue> {
        match self.get_index(item.get_primary_key()) {
            Ok(partition_index) => {
                let partition = self.partitions.get_mut(partition_index).unwrap();
                let (_, result) = partition.insert_or_replace(item);
                result
            }
            Err(index_to_insert) => {
                let sub_items = super::Partition::new(item);
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
    ) -> InsertIfNotExists2Keys<'s, TValue> {
        match self.get_index(primary_key) {
            Ok(partition_index) => {
                let partition = self.partitions.get_mut(partition_index).unwrap();
                match partition.binary_search(secondary_key) {
                    Ok(_) => InsertIfNotExists2Keys::Exists,
                    Err(row_index) => InsertIfNotExists2Keys::Insert(InsertEntity2Keys::new(
                        partition_index,
                        Some(row_index),
                        self,
                    )),
                }
            }
            Err(partition_index) => {
                InsertIfNotExists2Keys::Insert(InsertEntity2Keys::new(partition_index, None, self))
            }
        }
    }

    fn get_index(&self, primary_key: &str) -> Result<usize, usize> {
        self.partitions
            .binary_search_by(|itm| itm.get_key().cmp(primary_key))
    }

    pub fn insert_or_update<'s>(
        &'s mut self,
        primary_key: &str,
        secondary_key: &str,
    ) -> InsertOrUpdateEntry2Keys<'s, TValue> {
        let partition_index = self.get_index(primary_key);

        match partition_index {
            Ok(partition_index) => {
                let row_index = {
                    let partition = self.partitions.get_mut(partition_index).unwrap();
                    partition.binary_search(secondary_key)
                };

                match row_index {
                    Ok(row_index) => {
                        return InsertOrUpdateEntry2Keys::Update(UpdateEntry2Keys::new(
                            partition_index,
                            row_index,
                            self,
                        ))
                    }
                    Err(row_index) => {
                        return InsertOrUpdateEntry2Keys::Insert(InsertEntity2Keys::new(
                            partition_index,
                            Some(row_index),
                            self,
                        ));
                    }
                }
            }
            Err(partition_index) => {
                return InsertOrUpdateEntry2Keys::Insert(InsertEntity2Keys::new(
                    partition_index,
                    None,
                    self,
                ))
            }
        }
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

    pub fn get(&self, primary_key: &str, secondary_key: &str) -> Option<&TValue> {
        match self.get_index(primary_key) {
            Ok(partition_index) => {
                let partition = self.partitions.get(partition_index).unwrap();
                partition.get(secondary_key)
            }
            Err(_) => None,
        }
    }

    pub fn get_mut_or_create<'s>(
        &'s mut self,
        primary_key: &str,
        secondary_key: &str,
    ) -> GetMutOrCreateEntry2Keys<'s, TValue> {
        match self.get_index(primary_key) {
            Ok(partition_index) => {
                let row_index = {
                    let partition = self.partitions.get_mut(partition_index).unwrap();
                    partition.get_index(secondary_key)
                };

                match row_index {
                    Ok(row_index) => {
                        return GetMutOrCreateEntry2Keys::GetMut(
                            self.partitions
                                .get_mut(partition_index)
                                .unwrap()
                                .rows
                                .get_mut(row_index)
                                .unwrap(),
                        );
                    }
                    Err(row_index) => GetMutOrCreateEntry2Keys::Create(InsertEntity2Keys::new(
                        partition_index,
                        Some(row_index),
                        self,
                    )),
                }
            }
            Err(partition_index) => GetMutOrCreateEntry2Keys::Create(InsertEntity2Keys::new(
                partition_index,
                None,
                self,
            )),
        }
    }

    pub fn get_mut(&mut self, primary_key: &str, secondary_key: &str) -> Option<&mut TValue> {
        match self.get_index(primary_key) {
            Ok(partition_index) => {
                let partition = self.partitions.get_mut(partition_index).unwrap();
                partition.get_mut(secondary_key)
            }
            Err(_) => None,
        }
    }

    pub fn get_from_key_to_up(&self, primary_key: &str, secondary_key: &str) -> Option<&[TValue]> {
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
    ) -> Option<&[TValue]> {
        match self.get_index(primary_key) {
            Ok(partition_index) => {
                let partition = self.partitions.get(partition_index).unwrap();
                Some(partition.get_from_bottom_to_key(secondary_key))
            }
            Err(_) => None,
        }
    }

    pub fn remove(&mut self, primary_key: &str, secondary_key: &str) -> Option<TValue> {
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

    pub fn first(&self) -> Option<&TValue> {
        let partition = self.partitions.first()?;
        partition.first()
    }

    pub fn first_mut(&mut self) -> Option<&mut TValue> {
        let partition = self.partitions.first_mut()?;
        partition.first_mut()
    }

    pub fn last(&self) -> Option<&TValue> {
        let partition = self.partitions.last()?;
        partition.last()
    }

    pub fn last_mut(&mut self) -> Option<&mut TValue> {
        let partition = self.partitions.last_mut()?;
        partition.last_mut()
    }

    pub fn into_vec(self) -> Vec<TValue> {
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

    pub fn range(&self, primary_key: &str, range: std::ops::Range<&str>) -> Option<&[TValue]> {
        match self.get_index(primary_key) {
            Ok(partition_index) => {
                let partition = self.partitions.get(partition_index).unwrap();
                Some(partition.range(range))
            }
            Err(_) => None,
        }
    }

    pub fn remove_by_primary_key(&mut self, primary_key: &str) -> Option<Vec<TValue>> {
        match self.get_index(primary_key) {
            Ok(index) => {
                let partition = self.partitions.remove(index);
                self.len -= partition.len();
                Some(partition.into_vec())
            }
            Err(_) => None,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &TValue> {
        self.partitions
            .iter()
            .flat_map(|partition| partition.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut TValue> {
        self.partitions
            .iter_mut()
            .flat_map(|partition| partition.iter_mut())
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
        let mut items = SortedVecWith2StrKey::new();

        let item_to_insert = TestEntity {
            primary_key: "pk".to_string(),
            secondary_key: "key1".to_string(),
            value: 1,
        };

        match items.insert_or_if_not_exists(
            item_to_insert.get_primary_key(),
            item_to_insert.get_secondary_key(),
        ) {
            InsertIfNotExists2Keys::Insert(insert_entity) => {
                insert_entity.insert(item_to_insert);
            }
            InsertIfNotExists2Keys::Exists => {
                panic!("Should not be here")
            }
        }

        assert_eq!(items.len(), 1);
        assert_eq!(items.partitions_len(), 1);

        let item_to_insert = TestEntity {
            primary_key: "pk".to_string(),
            secondary_key: "key2".to_string(),
            value: 2,
        };

        match items.insert_or_if_not_exists(
            item_to_insert.get_primary_key(),
            item_to_insert.get_secondary_key(),
        ) {
            InsertIfNotExists2Keys::Insert(insert_entity) => {
                insert_entity.insert(item_to_insert);
            }
            InsertIfNotExists2Keys::Exists => {
                panic!("Should not be here")
            }
        }

        assert_eq!(items.len(), 2);
        assert_eq!(items.partitions_len(), 1);

        match items.insert_or_if_not_exists("pk", "key2") {
            InsertIfNotExists2Keys::Insert(_) => {
                panic!("Should not be here")
            }
            InsertIfNotExists2Keys::Exists => {}
        }

        let item = items.get("pk", "key2").unwrap();

        assert_eq!(item.value, 2)
    }
}
