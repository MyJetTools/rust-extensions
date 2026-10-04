use std::fmt::Debug;

use parking_lot::Mutex;

/// A `Copy` value that can be read and replaced through a shared reference
/// (`&self`), from any thread.
///
/// It used to write through a `&T` cast to `*mut T`. That is undefined behaviour
/// - the compiler may assume a value behind a shared reference never changes, and
/// release builds did drop such writes. The value now sits behind a short
/// `parking_lot` mutex: `get_value` copies it out, `set_value` replaces it. For a
/// plain flag or counter an `AtomicBool` / `AtomicUsize` is cheaper still.
#[derive(Default)]
pub struct UnsafeValue<T: Copy + Clone + Debug + Default> {
    value: Mutex<T>,
}

impl<T: Copy + Clone + Debug + Default> Debug for UnsafeValue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.get_value())
    }
}

impl<T: Copy + Clone + Debug + Default> UnsafeValue<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Mutex::new(value),
        }
    }

    pub fn get_value(&self) -> T {
        *self.value.lock()
    }

    pub fn set_value(&self, new_value: T) {
        *self.value.lock() = new_value;
    }
}

impl<T: Clone + Copy + Debug + Default> From<T> for UnsafeValue<T> {
    fn from(value: T) -> Self {
        UnsafeValue::new(value)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::UnsafeValue;

    #[test]
    fn test_change_value_unsafe() {
        let value: UnsafeValue<i32> = 10.into();

        assert_eq!(10, value.get_value());

        value.set_value(20);

        assert_eq!(20, value.get_value());
    }

    // The shape that lost writes in release builds: a flag behind an `Arc`, set
    // through `&self` by one function and read back by another.
    #[inline(never)]
    fn set_flag(flag: &UnsafeValue<bool>) {
        if !flag.get_value() {
            flag.set_value(true);
        }
    }

    #[test]
    fn a_value_set_through_a_shared_reference_is_seen_afterwards() {
        let flag = Arc::new(UnsafeValue::new(false));

        set_flag(&flag);
        assert!(flag.get_value());

        let other = flag.clone();
        std::thread::spawn(move || other.set_value(false))
            .join()
            .unwrap();
        assert!(!flag.get_value());
    }
}
