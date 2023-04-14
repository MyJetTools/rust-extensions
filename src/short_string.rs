use std::ops::Deref;

pub struct ShortString {
    data: [u8; 256],
}

impl ShortString {
    pub fn from_str(src: &str) -> Self {
        if src.len() > 255 {
            panic!(
                "ShortString is too long. Len must be no more than 255. Now it is {}",
                src.len()
            );
        }

        let mut data = [0u8; 256];
        data[1..src.len() + 1].copy_from_slice(src.as_bytes());
        data[0] = src.len() as u8;

        Self { data }
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
}

impl Deref for ShortString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
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
    }
}
