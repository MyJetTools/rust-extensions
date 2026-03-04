use std::{fmt::Debug, ops::Deref};

#[derive(Default)]
pub struct UnsafeValue<T: Copy + Clone + Debug + Default> {
    value: T,
}

impl<T: Copy + Clone + Debug + Default> Debug for UnsafeValue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.value)
    }
}

impl<T: Copy + Clone + Debug + Default> UnsafeValue<T> {
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

impl<T: Clone + Copy + Debug + Default> Deref for UnsafeValue<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Clone + Copy + Debug + Default> From<T> for UnsafeValue<T> {
    fn from(value: T) -> Self {
        UnsafeValue::new(value)
    }
}

#[cfg(test)]
mod tests {
    use crate::UnsafeValue;

    #[test]
    fn test_change_value_unsafe() {
        let value: UnsafeValue<i32> = 10.into();

        assert_eq!(10, value.get_value());

        value.set_value(20);

        assert_eq!(20, value.get_value());
    }
}
