use crate::EntityWithStrKey;

pub struct SortedVecWithStrKey<TValue: EntityWithStrKey> {
    items: Vec<TValue>,
}

impl<TValue: EntityWithStrKey> SortedVecWithStrKey<TValue> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn new_with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
        }
    }

    // Returns the index of the inserted item and old item if it was replaced
    pub fn insert_or_replace(&mut self, item: TValue) -> (usize, Option<TValue>) {
        let insert_index = self
            .items
            .binary_search_by(|itm| itm.get_key().cmp(item.get_key()));

        match insert_index {
            Ok(index) => {
                let got = std::mem::replace(&mut self.items[index], item);
                (index, Some(got))
            }
            Err(index) => {
                self.items.insert(index, item);
                (index, None)
            }
        }
    }

    pub fn insert_or_if_not_exists(
        &mut self,
        key: &str,
        new_item: impl Fn() -> TValue,
    ) -> Option<&TValue> {
        let insert_index = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match insert_index {
            Ok(_) => {
                return None;
            }
            Err(index) => {
                let item = new_item();
                self.items.insert(index, item);
                return self.items.get(index);
            }
        }
    }

    pub fn insert_or_update(
        &mut self,
        key: &str,
        new_item: impl Fn() -> TValue,
        update_item: impl Fn(&mut TValue),
    ) -> usize {
        let insert_index = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match insert_index {
            Ok(index) => {
                update_item(&mut self.items[index]);
                index
            }
            Err(index) => {
                let item = new_item();
                self.items.insert(index, item);
                index
            }
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));
        result.is_ok()
    }

    pub fn get(&self, key: &str) -> Option<&TValue> {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => self.items.get(index),
            Err(_) => None,
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut TValue> {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => self.items.get_mut(index),
            Err(_) => None,
        }
    }

    pub fn get_by_index(&self, index: usize) -> Option<&TValue> {
        self.items.get(index)
    }

    pub fn get_by_index_mut(&mut self, index: usize) -> Option<&mut TValue> {
        self.items.get_mut(index)
    }

    pub fn remove(&mut self, key: &str) -> Option<TValue> {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => Some(self.items.remove(index)),
            Err(_) => None,
        }
    }
    pub fn iter(&self) -> std::slice::Iter<TValue> {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<TValue> {
        self.items.iter_mut()
    }

    pub fn as_slice(&self) -> &[TValue] {
        self.items.as_slice()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

#[cfg(test)]
mod tests {

    use crate::EntityWithStrKey;

    #[derive(Debug)]
    pub struct TestEntity {
        pub key: String,
        pub value: u8,
    }

    impl EntityWithStrKey for TestEntity {
        fn get_key(&self) -> &str {
            &self.key
        }
    }

    #[test]
    fn test_cmp() {
        let src = vec![1, 2, 3, 5, 6];

        let result = src.binary_search(&4);

        println!("result: {:?}", result);
    }

    #[test]
    fn test_basic_inserts() {
        let mut vec = super::SortedVecWithStrKey::new();

        vec.insert_or_replace(TestEntity {
            key: "5".to_string(),
            value: 5,
        });

        vec.insert_or_replace(TestEntity {
            key: "4".to_string(),
            value: 4,
        });

        vec.insert_or_replace(TestEntity {
            key: "9".to_string(),
            value: 9,
        });

        assert_eq!(
            vec!["4", "5", "9"],
            vec.items.iter().map(|x| x.key.as_str()).collect::<Vec<_>>()
        );
    }
}
