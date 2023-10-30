use std::{
    fmt::{Debug, Display},
    ops::Deref,
};

#[derive(Clone)]
pub struct ShortString {
    data: [u8; 256],
}

impl ShortString {
    pub fn new_empty() -> Self {
        let data = [0u8; 256];
        Self { data }
    }

    pub fn from_str(src: &str) -> Self {
        let mut result = Self::new_empty();
        result.update(src);
        result
    }

    pub fn update(&mut self, src: &str) {
        if src.len() > 255 {
            panic!(
                "ShortString is too long. Len must be no more than 255. Now it is {}",
                src.len()
            );
        }

        self.data[1..src.len() + 1].copy_from_slice(src.as_bytes());
        self.data[0] = src.len() as u8;
    }

    pub fn len(&self) -> usize {
        self.data[0] as usize
    }

    pub fn push_str(&mut self, src: &str) {
        let len = self.len();

        let new_len = len + src.len();

        if new_len > 255 {
            panic!(
                "ShortString is too long. Len must be no more than 255. Now it is {}",
                new_len
            );
        }

        self.data[len + 1..len + src.len() + 1].copy_from_slice(src.as_bytes());
        self.data[0] = new_len as u8;
    }

    pub fn as_str(&self) -> &str {
        let len = self.len();

        unsafe { std::str::from_utf8_unchecked(&self.data[1..len + 1]) }
    }

    pub fn set_len(&mut self, pos: u8) {
        self.data[0] = pos;
    }
}

impl Deref for ShortString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl Display for ShortString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Debug for ShortString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShortString")
            .field("value", &self.as_str())
            .finish()
    }
}

impl<'s> Into<ShortString> for &'s String {
    fn into(self) -> ShortString {
        ShortString::from_str(self.as_str())
    }
}

impl<'s> Into<ShortString> for &'s str {
    fn into(self) -> ShortString {
        ShortString::from_str(self)
    }
}

#[cfg(test)]
mod test {
    use crate::ShortString;

    #[test]
    fn test_basic_cases() {
        let mut my_str = ShortString::from_str("Hello");

        assert_eq!(my_str.as_str(), "Hello");

        my_str.push_str(" world");

        assert_eq!(my_str.as_str(), "Hello world");

        println!("{}", my_str);
        println!("{:?}", my_str);
    }

    #[test]
    fn test_set_len() {
        let mut my_str = ShortString::from_str("Hello/");

        my_str.set_len(my_str.len() as u8 - 1);

        assert_eq!(my_str.as_str(), "Hello");
    }
}
