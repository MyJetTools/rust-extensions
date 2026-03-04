mod to_hash_map;
pub use to_hash_map::*;
mod to_btree_map;
pub use to_btree_map::*;

#[deprecated(note = "Please use to_hash_map or to_btree_map instead")]
pub fn to_hash_map<TSrcValue, TKey, TValue, TIter: Iterator<Item = TSrcValue>>(
    items: TIter,
    get_key_value: fn(TSrcValue) -> (TKey, TValue),
) -> std::collections::HashMap<TKey, TValue>
where
    TKey: std::cmp::Eq + core::hash::Hash + Clone,
{
    let mut result = std::collections::HashMap::new();

    for item in items {
        let (key, value) = get_key_value(item);
        result.insert(key, value);
    }

    result
}
