use std::sync::Arc;

use super::*;

pub enum InsertOrUpdateEntry2KeysArc<'s, TValue: EntityWith2StrKey> {
    Insert(InsertEntity2KeysArc<'s, TValue>),
    Update(UpdateEntry2KeysArc<'s, TValue>),
}

pub enum InsertIfNotExists2KeysArc<'s, TValue: EntityWith2StrKey> {
    Insert(InsertEntity2KeysArc<'s, TValue>),
    Exists,
}

pub enum GetMutOrCreateEntry2KeysArc<'s, TValue: EntityWith2StrKey> {
    GetMut(&'s mut Arc<TValue>),
    Create(InsertEntity2KeysArc<'s, TValue>),
}

pub enum GetOrCreateEntry2KeysArc<'s, TValue: EntityWith2StrKey> {
    Get(&'s Arc<TValue>),
    Create(InsertEntity2KeysArc<'s, TValue>),
}

pub struct InsertEntity2KeysArc<'s, TValue: EntityWith2StrKey> {
    partition_index: usize,
    row_index: Option<usize>,
    items: &'s mut SortedVecOfArcWith2StrKey<TValue>,
}

impl<'s, TValue: EntityWith2StrKey> InsertEntity2KeysArc<'s, TValue> {
    pub fn new(
        partition_index: usize,
        row_index: Option<usize>,
        items: &'s mut SortedVecOfArcWith2StrKey<TValue>,
    ) -> Self {
        Self {
            partition_index,
            row_index,
            items,
        }
    }

    pub fn insert(self, item: Arc<TValue>) {
        match self.row_index {
            Some(row_index) => {
                self.items
                    .insert_item_to_existing_partition(self.partition_index, row_index, item);
            }
            None => {
                let sub_items = PartitionOfArc::new(item);
                self.items
                    .insert_item_to_new_partition(self.partition_index, sub_items);
            }
        }
    }

    pub fn insert_and_get_value(self, item: Arc<TValue>) -> &'s Arc<TValue> {
        match self.row_index {
            Some(row_index) => {
                self.items
                    .insert_item_to_existing_partition(self.partition_index, row_index, item);

                self.items
                    .partitions
                    .get(self.partition_index)
                    .unwrap()
                    .rows
                    .get(row_index)
                    .unwrap()
            }
            None => {
                let sub_items = PartitionOfArc::new(item);
                self.items
                    .insert_item_to_new_partition(self.partition_index, sub_items);

                self.items
                    .partitions
                    .get(self.partition_index)
                    .unwrap()
                    .rows
                    .get(0)
                    .unwrap()
            }
        }
    }

    pub fn insert_and_get_value_mut(self, item: Arc<TValue>) -> &'s mut Arc<TValue> {
        match self.row_index {
            Some(row_index) => {
                self.items
                    .insert_item_to_existing_partition(self.partition_index, row_index, item);

                self.items
                    .partitions
                    .get_mut(self.partition_index)
                    .unwrap()
                    .rows
                    .get_mut(row_index)
                    .unwrap()
            }
            None => {
                let sub_items = PartitionOfArc::new(item);
                self.items
                    .insert_item_to_new_partition(self.partition_index, sub_items);

                self.items
                    .partitions
                    .get_mut(self.partition_index)
                    .unwrap()
                    .rows
                    .get_mut(0)
                    .unwrap()
            }
        }
    }
}

pub struct UpdateEntry2KeysArc<'s, TValue: EntityWith2StrKey> {
    pub primary_key_index: usize,
    pub secondary_key_index: usize,
    pub items: &'s mut SortedVecOfArcWith2StrKey<TValue>,
}

impl<'s, TValue: EntityWith2StrKey> UpdateEntry2KeysArc<'s, TValue> {
    pub fn new(
        primary_key_index: usize,
        secondary_key_index: usize,
        items: &'s mut SortedVecOfArcWith2StrKey<TValue>,
    ) -> Self {
        Self {
            primary_key_index,
            secondary_key_index,
            items,
        }
    }

    pub fn get_item_mut(&mut self) -> &mut Arc<TValue> {
        self.items
            .partitions
            .get_mut(self.primary_key_index)
            .unwrap()
            .rows
            .get_mut(self.secondary_key_index)
            .unwrap()
    }
}
