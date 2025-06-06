#[derive(Default)]
pub struct MinValue<T: Clone + Copy + PartialOrd> {
    value: Option<T>,
}

impl<T: Clone + Copy + PartialOrd> MinValue<T> {
    pub fn new() -> Self {
        Self { value: None }
    }

    pub fn get_value(&self) -> Option<T> {
        self.value
    }

    pub fn update(&mut self, value: T) {
        match self.value {
            Some(v) => {
                if value < v {
                    self.value = Some(value);
                }
            }
            None => {
                self.value = Some(value);
            }
        }
    }
}
