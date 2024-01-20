pub struct UnsafeValue<T: Copy + Clone> {
    value: T,
}

impl<T: Copy + Clone> UnsafeValue<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }

    pub fn get_value(&self) -> T {
        self.value
    }

    pub fn set_value(&self, new_value: T) {
        unsafe {
            let value = &self.value as *const T as *mut T;
            value.write(new_value);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::UnsafeValue;

    #[test]
    fn test_change_value_unsafe() {
        let value = UnsafeValue::new(10);

        assert_eq!(10, value.get_value());

        value.set_value(20);

        assert_eq!(20, value.get_value());
    }
}
