pub trait EntityWithBinarySearchKey<TKey> {
    fn get_key(&self) -> &TKey;

    fn compare_with_other_key(entity_key: &TKey, other_entity_key: &TKey) -> CmpOperation;
}

#[derive(Clone, Copy)]
pub enum CmpOperation {
    Greater,
    Lower,
    Eq,
}

impl CmpOperation {
    pub fn from_f64(entity_key: &f64, other_entity_key: &f64) -> Self {
        if other_entity_key > entity_key {
            return Self::Greater;
        }

        if other_entity_key < entity_key {
            return Self::Lower;
        }

        Self::Eq
    }
    pub fn from_f32(entity_key: &f32, other_entity_key: &f32) -> Self {
        if other_entity_key > entity_key {
            return Self::Greater;
        }

        if other_entity_key < entity_key {
            return Self::Lower;
        }

        Self::Eq
    }

    pub fn is_lower_or_equal(&self) -> bool {
        match self {
            CmpOperation::Greater => false,
            CmpOperation::Lower => true,
            CmpOperation::Eq => true,
        }
    }

    pub fn is_greater_or_equal(&self) -> bool {
        match self {
            CmpOperation::Greater => true,
            CmpOperation::Lower => false,
            CmpOperation::Eq => true,
        }
    }
}

pub fn binary_search<TKey: Clone, TItem: EntityWithBinarySearchKey<TKey>>(
    items: &[TItem],
    key: &TKey,
) -> Result<usize, usize> {
    if items.len() == 0 {
        return Err(0);
    }

    let first_item_key = items.first().unwrap().get_key();

    match TItem::compare_with_other_key(first_item_key, key) {
        CmpOperation::Greater => {}
        CmpOperation::Lower => return Err(0),
        CmpOperation::Eq => return Ok(0),
    }

    let last_item_key = items.last().unwrap().get_key();

    match TItem::compare_with_other_key(last_item_key, key) {
        CmpOperation::Greater => return Err(items.len()),
        CmpOperation::Lower => {}
        CmpOperation::Eq => return Ok(items.len() - 1),
    }

    let mut left_index = 0;
    let mut right_index = items.len();

    loop {
        {
            let items_amount = right_index - left_index;
            if items_amount <= 5 {
                let mut prev_item_key = items.get(left_index).unwrap().get_key();
                for i in left_index + 1..right_index + 1 {
                    let item_key = items.get(i).unwrap().get_key();

                    match TItem::compare_with_other_key(prev_item_key, key) {
                        CmpOperation::Greater => {
                            match TItem::compare_with_other_key(item_key, key) {
                                CmpOperation::Greater => {}
                                CmpOperation::Lower => return Err(i),
                                CmpOperation::Eq => return Ok(i),
                            }
                        }
                        CmpOperation::Lower => {}
                        CmpOperation::Eq => return Ok(i - 1),
                    }

                    prev_item_key = item_key;
                }
            }

            let middle_index = items_amount / 2 + left_index;

            let middle_index_key = items.get(middle_index).unwrap().get_key();
            match TItem::compare_with_other_key(middle_index_key, key) {
                CmpOperation::Greater => left_index = middle_index,
                CmpOperation::Lower => right_index = middle_index,
                CmpOperation::Eq => return Ok(middle_index),
            }

            continue;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CmpOperation, EntityWithBinarySearchKey};

    pub struct Entity {
        value: f64,
    }

    impl EntityWithBinarySearchKey<f64> for Entity {
        fn get_key(&self) -> &f64 {
            &self.value
        }
        fn compare_with_other_key(entity_key: &f64, other_entity_key: &f64) -> CmpOperation {
            CmpOperation::from_f64(entity_key, other_entity_key)
        }
    }

    #[test]
    fn test_inserting_with_no_record() {
        let items: Vec<Entity> = vec![];

        match super::binary_search(items.as_slice(), &1.0) {
            Ok(_) => {
                panic!("Should not be here");
            }
            Err(index) => {
                assert_eq!(index, 0);
            }
        }
    }

    #[test]
    fn test_inserting_with_single_record() {
        let items: Vec<Entity> = vec![Entity { value: 2.0 }];

        match super::binary_search(items.as_slice(), &1.0) {
            Ok(_) => {
                panic!("Should not be here");
            }
            Err(index) => {
                assert_eq!(index, 0);
            }
        }
        match super::binary_search(items.as_slice(), &3.0) {
            Ok(_) => {
                panic!("Should not be here");
            }
            Err(index) => {
                assert_eq!(index, 1);
            }
        }

        match super::binary_search(items.as_slice(), &2.0) {
            Ok(index) => {
                assert_eq!(index, 0);
            }
            Err(_) => {
                panic!("Should not be here")
            }
        }
    }

    #[test]
    fn test_inserting_with_two_records() {
        let items: Vec<Entity> = vec![Entity { value: 2.0 }, Entity { value: 4.0 }];

        match super::binary_search(items.as_slice(), &1.0) {
            Ok(_) => {
                panic!("Should not be here");
            }
            Err(index) => {
                assert_eq!(index, 0);
            }
        }
        match super::binary_search(items.as_slice(), &2.0) {
            Ok(index) => {
                assert_eq!(index, 0);
            }
            Err(_) => {
                panic!("Should not be here")
            }
        }

        match super::binary_search(items.as_slice(), &3.0) {
            Ok(_) => {
                panic!("Should not be here");
            }
            Err(index) => {
                assert_eq!(index, 1);
            }
        }

        match super::binary_search(items.as_slice(), &4.0) {
            Ok(index) => {
                assert_eq!(index, 1);
            }
            Err(_) => {
                panic!("Should not be here")
            }
        }

        match super::binary_search(items.as_slice(), &5.0) {
            Ok(_) => {
                panic!("Should not be here")
            }
            Err(index) => {
                assert_eq!(index, 2);
            }
        }
    }

    #[test]
    fn test_inserting_with_three_records() {
        let items: Vec<Entity> = vec![
            Entity { value: 2.0 },
            Entity { value: 4.0 },
            Entity { value: 6.0 },
        ];

        match super::binary_search(items.as_slice(), &1.0) {
            Ok(_) => {
                panic!("Should not be here");
            }
            Err(index) => {
                assert_eq!(index, 0);
            }
        }
        match super::binary_search(items.as_slice(), &2.0) {
            Ok(index) => {
                assert_eq!(index, 0);
            }
            Err(_) => {
                panic!("Should not be here")
            }
        }

        match super::binary_search(items.as_slice(), &3.0) {
            Ok(_) => {
                panic!("Should not be here");
            }
            Err(index) => {
                assert_eq!(index, 1);
            }
        }

        match super::binary_search(items.as_slice(), &4.0) {
            Ok(index) => {
                assert_eq!(index, 1);
            }
            Err(_) => {
                panic!("Should not be here")
            }
        }

        match super::binary_search(items.as_slice(), &5.0) {
            Ok(_) => {
                panic!("Should not be here")
            }
            Err(index) => {
                assert_eq!(index, 2);
            }
        }

        match super::binary_search(items.as_slice(), &6.0) {
            Ok(index) => {
                assert_eq!(index, 2);
            }
            Err(_) => {
                panic!("Should not be here")
            }
        }

        match super::binary_search(items.as_slice(), &7.0) {
            Ok(_) => {
                panic!("Should not be here")
            }
            Err(index) => {
                assert_eq!(index, 3);
            }
        }
    }

    #[test]
    fn test_inserting_with_11_records() {
        let items: Vec<Entity> = vec![
            Entity { value: 2.0 },
            Entity { value: 4.0 },
            Entity { value: 6.0 },
            Entity { value: 8.0 },
            Entity { value: 10.0 },
            Entity { value: 12.0 },
            Entity { value: 14.0 },
            Entity { value: 16.0 },
            Entity { value: 18.0 },
            Entity { value: 20.0 },
            Entity { value: 22.0 },
        ];

        let mut key = 1.0;

        let mut exact = false;
        let mut vec_index: usize = 0;
        while key <= 23.0 {
            let search_result = super::binary_search(items.as_slice(), &key);
            if exact {
                match search_result {
                    Ok(index) => {
                        assert_eq!(index, vec_index);
                    }
                    Err(_) => {
                        panic!("Should not be here");
                    }
                }
                vec_index += 1;
            } else {
                match search_result {
                    Ok(_) => {
                        panic!("Should not be here");
                    }
                    Err(index) => {
                        assert_eq!(index, vec_index);
                    }
                }
            }

            exact = !exact;
            key += 1.0;
        }
    }
}
