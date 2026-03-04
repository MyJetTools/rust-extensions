pub trait EntityWithBinarySearchKey<TKey: PartialOrd + Copy + Clone> {
    fn get_key(&self) -> &TKey;
}

pub fn binary_search<TKey: Clone + Copy + PartialOrd, TItem: EntityWithBinarySearchKey<TKey>>(
    items: &[TItem],
    key: TKey,
) -> Result<usize, usize> {
    if items.len() == 0 {
        return Err(0);
    }

    let first_item_key = *items.first().unwrap().get_key();

    if first_item_key == key {
        return Ok(0);
    }

    if key < first_item_key {
        return Err(0);
    }

    let last_item_key = *items.last().unwrap().get_key();

    if last_item_key == key {
        return Ok(items.len() - 1);
    }

    if key > last_item_key {
        return Err(items.len());
    }

    let mut left_index = 0;
    let mut right_index = items.len();

    loop {
        {
            let items_amount = right_index - left_index;
            if items_amount <= 5 {
                let mut prev_item_key = *items.get(left_index).unwrap().get_key();

                for i in left_index + 1..right_index + 1 {
                    if prev_item_key == key {
                        return Ok(i - 1);
                    }
                    let item_key = *items.get(i).unwrap().get_key();

                    if item_key == key {
                        return Ok(i);
                    }

                    if prev_item_key < key && key < item_key {
                        return Err(i);
                    }

                    prev_item_key = item_key;
                }
            }

            let middle_index = items_amount / 2 + left_index;

            let middle_index_key = *items.get(middle_index).unwrap().get_key();

            if key < middle_index_key {
                right_index = middle_index
            } else if key > middle_index_key {
                left_index = middle_index
            } else {
                return Ok(middle_index);
            }

            continue;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EntityWithBinarySearchKey;

    pub struct Entity {
        value: f64,
    }

    impl EntityWithBinarySearchKey<f64> for Entity {
        fn get_key(&self) -> &f64 {
            &self.value
        }
    }

    #[test]
    fn test_inserting_with_no_record() {
        let items: Vec<Entity> = vec![];

        match super::binary_search(items.as_slice(), 1.0) {
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

        match super::binary_search(items.as_slice(), 1.0) {
            Ok(_) => {
                panic!("Should not be here");
            }
            Err(index) => {
                assert_eq!(index, 0);
            }
        }
        match super::binary_search(items.as_slice(), 3.0) {
            Ok(_) => {
                panic!("Should not be here");
            }
            Err(index) => {
                assert_eq!(index, 1);
            }
        }

        match super::binary_search(items.as_slice(), 2.0) {
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

        match super::binary_search(items.as_slice(), 1.0) {
            Ok(_) => {
                panic!("Should not be here");
            }
            Err(index) => {
                assert_eq!(index, 0);
            }
        }
        match super::binary_search(items.as_slice(), 2.0) {
            Ok(index) => {
                assert_eq!(index, 0);
            }
            Err(_) => {
                panic!("Should not be here")
            }
        }

        match super::binary_search(items.as_slice(), 3.0) {
            Ok(_) => {
                panic!("Should not be here");
            }
            Err(index) => {
                assert_eq!(index, 1);
            }
        }

        match super::binary_search(items.as_slice(), 4.0) {
            Ok(index) => {
                assert_eq!(index, 1);
            }
            Err(_) => {
                panic!("Should not be here")
            }
        }

        match super::binary_search(items.as_slice(), 5.0) {
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

        match super::binary_search(items.as_slice(), 1.0) {
            Ok(_) => {
                panic!("Should not be here");
            }
            Err(index) => {
                assert_eq!(index, 0);
            }
        }
        match super::binary_search(items.as_slice(), 2.0) {
            Ok(index) => {
                assert_eq!(index, 0);
            }
            Err(_) => {
                panic!("Should not be here")
            }
        }

        match super::binary_search(items.as_slice(), 3.0) {
            Ok(_) => {
                panic!("Should not be here");
            }
            Err(index) => {
                assert_eq!(index, 1);
            }
        }

        match super::binary_search(items.as_slice(), 4.0) {
            Ok(index) => {
                assert_eq!(index, 1);
            }
            Err(_) => {
                panic!("Should not be here")
            }
        }

        match super::binary_search(items.as_slice(), 5.0) {
            Ok(_) => {
                panic!("Should not be here")
            }
            Err(index) => {
                assert_eq!(index, 2);
            }
        }

        match super::binary_search(items.as_slice(), 6.0) {
            Ok(index) => {
                assert_eq!(index, 2);
            }
            Err(_) => {
                panic!("Should not be here")
            }
        }

        match super::binary_search(items.as_slice(), 7.0) {
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
            let search_result = super::binary_search(items.as_slice(), key);
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
