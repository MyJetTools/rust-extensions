use crate::EntityWithKey;

pub enum InsertIndex {
    Replace(usize),
    Insert(usize),
}

pub struct SortedVec<TKey: Ord, TValue: EntityWithKey<TKey>> {
    items: Vec<TValue>,
    itm: std::marker::PhantomData<TKey>,
}

impl<TKey: Ord, TValue: EntityWithKey<TKey>> SortedVec<TKey, TValue> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            itm: std::marker::PhantomData,
        }
    }

    pub fn new_with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            itm: std::marker::PhantomData,
        }
    }

    pub fn insert_or_replace(&mut self, item: TValue) -> Option<TValue> {
        let insert_index = self
            .items
            .binary_search_by(|itm| itm.get_key().cmp(item.get_key()));

        match insert_index {
            Ok(index) => {
                let got = std::mem::replace(&mut self.items[index], item);
                Some(got)
            }
            Err(index) => {
                self.items.insert(index, item);
                None
            }
        }
    }

    pub fn find(&mut self, key: &TKey) -> Option<TValue> {
        let result = self.items.binary_search_by(|itm| itm.get_key().cmp(key));

        match result {
            Ok(index) => Some(self.items.remove(index)),
            Err(_) => None,
        }
    }

    pub fn remove(&mut self, key: &TKey) -> Option<TValue> {
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

    use crate::EntityWithKey;

    #[derive(Debug)]
    pub struct TestEntity {
        pub key: u8,
        pub value: u8,
    }

    impl EntityWithKey<u8> for TestEntity {
        fn get_key(&self) -> &u8 {
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
        let mut vec = super::SortedVec::new();

        vec.insert_or_replace(TestEntity { key: 5, value: 5 });

        vec.insert_or_replace(TestEntity { key: 4, value: 6 });

        vec.insert_or_replace(TestEntity { key: 9, value: 9 });

        assert_eq!(
            vec![4u8, 5u8, 9u8],
            vec.items.iter().map(|x| x.key).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_insert_third_is_first() {
        let mut vec = super::SortedVec::new();

        vec.insert_or_replace(TestEntity { key: 5, value: 5 });

        vec.insert_or_replace(TestEntity { key: 4, value: 4 });

        vec.insert_or_replace(TestEntity { key: 3, value: 3 });

        vec.insert_or_replace(TestEntity { key: 1, value: 1 });

        vec.insert_or_replace(TestEntity { key: 2, value: 2 });

        assert_eq!(
            vec![1u8, 2u8, 3u8, 4u8, 5u8],
            vec.items.iter().map(|x| x.key).collect::<Vec<_>>()
        );
    }
}
