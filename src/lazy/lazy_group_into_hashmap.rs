use std::collections::HashMap;

pub struct LazyGroupIntoHashMap<TKey, TValue>
where
    TKey: std::cmp::Eq + core::hash::Hash + Clone,
{
    items: Option<HashMap<TKey, Vec<TValue>>>,
}

impl<TKey, TValue> LazyGroupIntoHashMap<TKey, TValue>
where
    TKey: std::cmp::Eq + core::hash::Hash + Clone,
{
    pub fn new() -> Self {
        Self { items: None }
    }

    pub fn add(&mut self, key: &TKey, value: TValue) {
        if self.items.is_none() {
            self.items = Some(HashMap::new());
        }

        let items = self.items.as_mut().unwrap();

        if !items.contains_key(key) {
            items.insert(key.to_owned(), Vec::new());
        }
        items.get_mut(key).unwrap().push(value);
    }

    pub fn get_result(self) -> Option<HashMap<TKey, Vec<TValue>>> {
        self.items
    }
}
