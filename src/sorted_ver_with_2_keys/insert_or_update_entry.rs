use super::*;

pub enum InsertOrUpdateEntry2Keys<'s, TValue: EntityWith2StrKey> {
    Insert(InsertEntity2Keys<'s, TValue>),
    Update(UpdateEntry2Keys<'s, TValue>),
}

pub enum InsertIfNotExists2Keys<'s, TValue: EntityWith2StrKey> {
    Insert(InsertEntity2Keys<'s, TValue>),
    Exists,
}

pub enum GetMutOrCreateEntry2Keys<'s, TValue: EntityWith2StrKey> {
    GetMut(&'s mut TValue),
    Create(InsertEntity2Keys<'s, TValue>),
}

pub enum GetOrCreateEntry2Keys<'s, TValue: EntityWith2StrKey> {
    Get(&'s TValue),
    Create(InsertEntity2Keys<'s, TValue>),
}
pub struct InsertEntity2Keys<'s, TValue: EntityWith2StrKey> {
    partition_index: usize,
    row_index: Option<usize>,
    items: &'s mut SortedVecWith2StrKey<TValue>,
}

impl<'s, TValue: EntityWith2StrKey> InsertEntity2Keys<'s, TValue> {
    pub fn new(
        partition_index: usize,
        row_index: Option<usize>,
        items: &'s mut SortedVecWith2StrKey<TValue>,
    ) -> Self {
        Self {
            partition_index,
            row_index,
            items,
        }
    }

    pub fn insert(self, item: TValue) {
        match self.row_index {
            Some(row_index) => {
                self.items
                    .insert_item_to_existing_partition(self.partition_index, row_index, item);
            }
            None => {
                let sub_items = Partition::new(item);
                self.items
                    .insert_item_to_new_partition(self.partition_index, sub_items);
            }
        }
    }

    pub fn insert_and_get_value(self, item: TValue) -> &'s TValue {
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
                let sub_items = Partition::new(item);
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

    pub fn insert_and_get_value_mut(self, item: TValue) -> &'s mut TValue {
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
                let sub_items = Partition::new(item);
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

pub struct UpdateEntry2Keys<'s, TValue: EntityWith2StrKey> {
    pub primary_key_index: usize,
    pub secondary_key_index: usize,
    pub items: &'s mut SortedVecWith2StrKey<TValue>,
}

impl<'s, TValue: EntityWith2StrKey> UpdateEntry2Keys<'s, TValue> {
    pub fn new(
        primary_key_index: usize,
        secondary_key_index: usize,
        items: &'s mut SortedVecWith2StrKey<TValue>,
    ) -> Self {
        Self {
            primary_key_index,
            secondary_key_index,
            items,
        }
    }

    pub fn get_item_mut(&mut self) -> &mut TValue {
        self.items
            .partitions
            .get_mut(self.primary_key_index)
            .unwrap()
            .rows
            .get_mut(self.secondary_key_index)
            .unwrap()
    }
}
