use std::{fmt::Debug, ops::Deref};

pub struct UnsafeValue<T: Copy + Clone + Debug> {
    value: T,
}

impl<T: Copy + Clone + Debug> Debug for UnsafeValue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.value)
    }
}

impl<T: Copy + Clone + Debug> UnsafeValue<T> {
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

impl<T: Clone + Copy + Debug> Deref for UnsafeValue<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl Into<UnsafeValue<bool>> for bool {
    fn into(self) -> UnsafeValue<bool> {
        UnsafeValue::new(self)
    }
}

impl Into<UnsafeValue<u8>> for u8 {
    fn into(self) -> UnsafeValue<u8> {
        UnsafeValue::new(self)
    }
}

impl Into<UnsafeValue<i8>> for i8 {
    fn into(self) -> UnsafeValue<i8> {
        UnsafeValue::new(self)
    }
}

impl Into<UnsafeValue<u16>> for u16 {
    fn into(self) -> UnsafeValue<u16> {
        UnsafeValue::new(self)
    }
}

impl Into<UnsafeValue<i16>> for i16 {
    fn into(self) -> UnsafeValue<i16> {
        UnsafeValue::new(self)
    }
}

impl Into<UnsafeValue<u32>> for u32 {
    fn into(self) -> UnsafeValue<u32> {
        UnsafeValue::new(self)
    }
}

impl Into<UnsafeValue<i32>> for i32 {
    fn into(self) -> UnsafeValue<i32> {
        UnsafeValue::new(self)
    }
}

impl Into<UnsafeValue<u64>> for u64 {
    fn into(self) -> UnsafeValue<u64> {
        UnsafeValue::new(self)
    }
}

impl Into<UnsafeValue<i64>> for i64 {
    fn into(self) -> UnsafeValue<i64> {
        UnsafeValue::new(self)
    }
}

impl Into<UnsafeValue<usize>> for usize {
    fn into(self) -> UnsafeValue<usize> {
        UnsafeValue::new(self)
    }
}

impl Into<UnsafeValue<isize>> for isize {
    fn into(self) -> UnsafeValue<isize> {
        UnsafeValue::new(self)
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
