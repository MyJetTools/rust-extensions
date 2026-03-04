use std::collections::HashMap;

pub fn group_to_hash_map<TKey, TValue>(
    src: impl Iterator<Item = TValue>,
    get_key: impl Fn(&TValue) -> &TKey,
) -> HashMap<TKey, Vec<TValue>>
where
    TKey: std::cmp::Eq + core::hash::Hash + Clone,
{
    let mut result: HashMap<TKey, Vec<TValue>> = HashMap::new();

    for itm in src {
        let key = get_key(&itm);

        match result.get_mut(key) {
            Some(items) => {
                items.push(itm);
            }
            None => {
                result.insert(key.clone(), vec![itm]);
            }
        }
    }

    result
}

pub struct GroupedDataAsHashmap<TGroupKey, TKey, TValue>
where
    TGroupKey: std::cmp::Eq + core::hash::Hash + Clone,
    TKey: std::cmp::Eq + core::hash::Hash + Clone,
{
    pub items: HashMap<TGroupKey, HashMap<TKey, TValue>>,
}

impl<TGroupKey, TKey, TValue> GroupedDataAsHashmap<TGroupKey, TKey, TValue>
where
    TGroupKey: std::cmp::Eq + core::hash::Hash + Clone,
    TKey: std::cmp::Eq + core::hash::Hash + Clone,
{
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    pub fn insert(&mut self, group_key: &TGroupKey, key: TKey, value: TValue) -> Option<TValue> {
        if !self.items.contains_key(group_key) {
            self.items.insert(group_key.to_owned(), HashMap::new());
        }

        self.items.get_mut(group_key).unwrap().insert(key, value)
    }

    pub fn remove(&mut self, group_key: &TGroupKey, key: &TKey) -> Option<TValue> {
        if !self.items.contains_key(group_key) {
            return None;
        }

        let (items_after_delete, result) = {
            let items = self.items.get_mut(group_key).unwrap();
            let result = items.remove(key);

            (items.len(), result)
        };

        if items_after_delete == 0 {
            self.items.remove(group_key);
        }

        result
    }

    pub fn get_data_by_group(&self, group_key: &TGroupKey) -> Option<&HashMap<TKey, TValue>> {
        self.items.get(group_key)
    }
}
