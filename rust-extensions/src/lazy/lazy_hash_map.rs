use std::collections::HashMap;

pub struct LazyHashMap<TKey: std::cmp::Eq + core::hash::Hash, TValue> {
    data: Option<HashMap<TKey, TValue>>,
}

impl<TKey: std::cmp::Eq + core::hash::Hash, TValue> LazyHashMap<TKey, TValue> {
    pub fn new() -> Self {
        Self { data: None }
    }

    fn get_hash_map_mut(&mut self) -> &mut HashMap<TKey, TValue> {
        if self.data.is_none() {
            self.data = Some(HashMap::new());
        }

        return self.data.as_mut().unwrap();
    }

    pub fn insert(&mut self, key: TKey, value: TValue) -> Option<TValue> {
        self.get_hash_map_mut().insert(key, value)
    }

    pub fn get_result(self) -> Option<HashMap<TKey, TValue>> {
        self.data
    }
}
