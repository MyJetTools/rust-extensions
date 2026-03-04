#[derive(Default)]
pub struct MinKeyValue<TKey: Clone + Copy + PartialOrd, TValue: Clone> {
    value: Option<(TKey, TValue)>,
}

impl<TKey: Clone + Copy + PartialOrd, TValue: Clone> MinKeyValue<TKey, TValue> {
    pub fn new() -> Self {
        Self { value: None }
    }

    pub fn get_value(&self) -> Option<(TKey, TValue)> {
        self.value.clone()
    }

    pub fn update(&mut self, key: TKey, value: TValue) {
        match &self.value {
            Some(k_v) => {
                if key < k_v.0 {
                    self.value = Some((key, value));
                }
            }
            None => {
                self.value = Some((key, value));
            }
        }
    }
}
