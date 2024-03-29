use std::collections::{BTreeMap, HashMap};

pub fn group_to_btree_map<TKey, TValue, TIterator: Iterator<Item = TValue>, TGetKey>(
    src: TIterator,
    get_key: TGetKey,
) -> BTreeMap<TKey, Vec<TValue>>
where
    TKey: Ord + Clone,
    TGetKey: Fn(&TValue) -> TKey,
{
    let mut result = BTreeMap::new();

    for itm in src {
        let key = get_key(&itm);

        if !result.contains_key(&key) {
            result.insert(key.clone(), Vec::new());
        }

        result.get_mut(&key).unwrap().push(itm);
    }

    result
}

pub struct GroupedDataAsBTreeMap<TGroupKey, TKey, TValue>
where
    TGroupKey: std::cmp::Eq + core::hash::Hash + Ord + Clone,
    TKey: std::cmp::Eq + core::hash::Hash + Clone,
{
    pub items: BTreeMap<TGroupKey, HashMap<TKey, TValue>>,
}

impl<TGroupKey, TKey, TValue> GroupedDataAsBTreeMap<TGroupKey, TKey, TValue>
where
    TGroupKey: std::cmp::Eq + core::hash::Hash + Ord + Clone,
    TKey: std::cmp::Eq + core::hash::Hash + Clone,
{
    pub fn new() -> Self {
        Self {
            items: BTreeMap::new(),
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
