use std::collections::HashMap;

pub struct ToHashMapConverter<TSrcValue, TKey, TValue, TConvertor>
where
    TKey: std::cmp::Eq + core::hash::Hash + Clone,
    TConvertor: Fn(TSrcValue) -> Option<(TKey, TValue)>,
{
    convertor: Option<TConvertor>,
    src: Vec<TSrcValue>,
}

impl<TSrcValue, TKey, TValue, TConvertor: Fn(TSrcValue) -> Option<(TKey, TValue)>>
    ToHashMapConverter<TSrcValue, TKey, TValue, TConvertor>
where
    TKey: std::cmp::Eq + core::hash::Hash + Clone,
    TConvertor: Fn(TSrcValue) -> Option<(TKey, TValue)>,
{
    pub fn new(src: Vec<TSrcValue>, convertor: TConvertor) -> Self {
        Self {
            convertor: Some(convertor),
            src,
        }
    }

    pub fn collect(mut self) -> HashMap<TKey, TValue> {
        let mut result = HashMap::new();

        let convertor = self.convertor.take().unwrap();

        for item in self.src {
            if let Some((key, value)) = convertor(item) {
                result.insert(key, value);
            }
        }

        result
    }
}

pub trait ToHashMap<TSrcValue, TKey, TValue, TConvertor: Fn(TValue) -> Option<(TKey, TValue)>>
where
    TKey: std::cmp::Eq + core::hash::Hash + Clone,
    TConvertor: Fn(TSrcValue) -> Option<(TKey, TValue)>,
{
    fn to_hash_map(
        self,
        convertor: TConvertor,
    ) -> ToHashMapConverter<TSrcValue, TKey, TValue, TConvertor>;
}

impl<TSrcValue, TKey, TValue, TConvertor: Fn(TValue) -> Option<(TKey, TValue)>>
    ToHashMap<TSrcValue, TKey, TValue, TConvertor> for Vec<TSrcValue>
where
    TKey: std::cmp::Eq + core::hash::Hash + Clone,
    TConvertor: Fn(TSrcValue) -> Option<(TKey, TValue)>,
{
    fn to_hash_map(
        self,
        convertor: TConvertor,
    ) -> ToHashMapConverter<TSrcValue, TKey, TValue, TConvertor> {
        ToHashMapConverter::new(self, convertor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_hash_map() {
        let items = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let result = items
            .to_hash_map(|item: i32| Some((item.to_string(), item)))
            .collect();

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
