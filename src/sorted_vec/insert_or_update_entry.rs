pub enum InsertOrUpdateEntry<'s, TValue> {
    Insert(InsertEntity<'s, TValue>),
    Update(UpdateEntry<'s, TValue>),
}

pub enum GetMutOrCreateEntry<'s, TValue> {
    GetMut(&'s mut TValue),
    Create(InsertEntity<'s, TValue>),
}

pub struct InsertEntity<'s, TValue> {
    index: usize,
    items: &'s mut Vec<TValue>,
}

impl<'s, TValue> InsertEntity<'s, TValue> {
    pub fn new(index: usize, items: &'s mut Vec<TValue>) -> Self {
        Self { index, items }
    }

    pub fn insert(self, item: TValue) -> usize {
        self.items.insert(self.index, item);
        self.index
    }
}

pub struct UpdateEntry<'s, TValue> {
    pub index: usize,
    pub item: &'s mut TValue,
}

impl<'s, TValue> UpdateEntry<'s, TValue> {
    pub fn new(index: usize, item: &'s mut TValue) -> Self {
        Self { index, item }
    }
}
