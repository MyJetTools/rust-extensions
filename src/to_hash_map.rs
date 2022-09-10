use std::collections::HashMap;

pub trait ToHashMap<TKey, TValue> {
    fn to_hash_map(self, get_key: fn(&TValue) -> TKey) -> HashMap<TKey, TValue>;
}

impl<TKey, TValue> ToHashMap<TKey, TValue> for Vec<TValue>
where
    TKey: std::cmp::Eq + core::hash::Hash + Clone,
{
    fn to_hash_map(self, get_key: fn(&TValue) -> TKey) -> HashMap<TKey, TValue> {
        to_hash_map(self.into_iter(), get_key)
    }
}

pub fn to_hash_map<TKey, TValue, TIter: Iterator<Item = TValue>>(
    items: TIter,
    get_key: fn(&TValue) -> TKey,
) -> HashMap<TKey, TValue>
where
    TKey: std::cmp::Eq + core::hash::Hash + Clone,
{
    let mut result = HashMap::new();

    for item in items {
        result.insert(get_key(&item), item);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_hash_map() {
        let items = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let result = to_hash_map(items.into_iter(), |item| item.to_string());

        assert_eq!(result.len(), 10);

        for no in 1..11 {
            let key = no.to_string();
            if let Some(value) = result.get(key.as_str()) {
                assert_eq!(*value, no);
            } else {
                panic!("Value not found");
            }
        }
    }
}
