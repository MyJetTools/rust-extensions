use std::collections::BTreeMap;

pub struct LazyGroupIntoBTreeMap<TKey: std::cmp::Eq + core::hash::Hash + Ord + Clone, TValue> {
    items: Option<BTreeMap<TKey, Vec<TValue>>>,
}

impl<TKey: std::cmp::Eq + core::hash::Hash + Clone + Ord, TValue>
    LazyGroupIntoBTreeMap<TKey, TValue>
{
    pub fn new() -> Self {
        Self { items: None }
    }

    pub fn add(&mut self, key: &TKey, value: TValue) {
        if self.items.is_none() {
            self.items = Some(BTreeMap::new());
        }

        let items = self.items.as_mut().unwrap();

        if !items.contains_key(key) {
            items.insert(key.to_owned(), Vec::new());
        }
        items.get_mut(key).unwrap().push(value);
    }

    pub fn get_result(self) -> Option<BTreeMap<TKey, Vec<TValue>>> {
        self.items
    }
}
