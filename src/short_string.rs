use std::{
    fmt::{Debug, Display},
    ops::Deref,
};

use crate::slice_of_u8_utils::SliceOfU8Ext;

pub const SHORT_STRING_MAX_LEN: usize = 255;
#[derive(Clone)]
pub struct ShortString {
    data: [u8; 256],
}

impl ShortString {
    pub fn new_empty() -> Self {
        let data = [0u8; 256];
        Self { data }
    }

    pub fn from_str(src: &str) -> Option<Self> {
        if src.len() > SHORT_STRING_MAX_LEN {
            return None;
        }

        let mut result = Self::new_empty();
        result.update(src);
        Some(result)
    }

    pub fn from_str_convert_to_lower_case(src: &str) -> Option<Self> {
        if src.len() > SHORT_STRING_MAX_LEN {
            return None;
        }

        let mut result = Self::new_empty();
        for c in src.chars() {
            result.push(c.to_ascii_lowercase());
        }
        Some(result)
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

    pub fn push(&mut self, c: char) {
        let len = self.len();

        let new_len = len + 1;

        if new_len > 255 {
            panic!(
                "ShortString is too long. Len must be no more than 255. Now it is {}",
                new_len
            );
        }

        self.data[len + 1] = c as u8;
        self.data[0] = new_len as u8;
    }

    pub fn as_str(&self) -> &str {
        let len = self.len();

        unsafe { std::str::from_utf8_unchecked(&self.data[1..len + 1]) }
    }

    pub fn set_len(&mut self, pos: u8) {
        self.data[0] = pos;
    }

    pub fn compare_with_case_insensitive(&self, other: &str) -> bool {
        crate::str_utils::compare_strings_case_insensitive(self.as_str(), other)
    }

    pub fn replace(&mut self, from: &str, to: &str) -> bool {
        let mut pos = 0;

        while let Some(found_pos) = (&self.data[1..]).find_sequence_pos(from.as_bytes(), pos) {
            if self.len() - from.len() + to.len() > SHORT_STRING_MAX_LEN {
                return false;
            }

            if from.len() == to.len() {
                self.data[found_pos + 1..found_pos + 1 + from.len()].copy_from_slice(to.as_bytes());
            } else {
                let (pos_from_move, pos_to_move, new_len) = if from.len() < to.len() {
                    let size_increase = to.len() - from.len();
                    let pos_from_move = found_pos + from.len() + 1;
                    let pos_to_move = pos_from_move + size_increase;

                    (pos_from_move, pos_to_move, self.len() + size_increase)
                } else {
                    let size_decrease = from.len() - to.len();
                    let pos_from_move = found_pos + from.len() + 1;
                    let pos_to_move = pos_from_move - size_decrease;
                    (pos_from_move, pos_to_move, self.len() - size_decrease)
                };

                let mut slice_to_copy = [0u8; 255];

                let len_to_copy = self.len() + 1 - pos_from_move;

                slice_to_copy[..len_to_copy]
                    .copy_from_slice(&self.data[pos_from_move..pos_from_move + len_to_copy]);

                self.data[pos_to_move..pos_to_move + len_to_copy]
                    .copy_from_slice(&slice_to_copy[..len_to_copy]);

                let to = to.as_bytes();

                self.data[found_pos + 1..found_pos + 1 + to.len()].copy_from_slice(to);

                self.data[0] = new_len as u8;
            }

            pos += to.len();
        }

        true
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
        match ShortString::from_str(self.as_str()) {
            Some(result) => result,
            None => panic!("Can not convert String to ShortString. The Size of the string mist be {} bytes or less. Apparently it is {}", SHORT_STRING_MAX_LEN, self.len()),
        }
    }
}

impl<'s> Into<ShortString> for &'s str {
    fn into(self) -> ShortString {
        match ShortString::from_str(self) {
            Some(result) => result,
            None => panic!("Can not convert String to ShortString. The Size of the string mist be {} bytes or less. Apparently it is {}", SHORT_STRING_MAX_LEN, self.len()),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ShortString;

    #[test]
    fn test_basic_cases() {
        let mut my_str = ShortString::from_str("Hello").unwrap();

        assert_eq!(my_str.as_str(), "Hello");

        my_str.push_str(" world");

        assert_eq!(my_str.as_str(), "Hello world");

        println!("{}", my_str);
        println!("{:?}", my_str);
    }

    #[test]
    fn test_set_len() {
        let mut my_str = ShortString::from_str("Hello/").unwrap();

        my_str.set_len(my_str.len() as u8 - 1);

        assert_eq!(my_str.as_str(), "Hello");
    }

    #[test]
    fn test_replace_the_same_size() {
        let mut my_str = ShortString::from_str("Hello my world my").unwrap();

        my_str.replace("my", "ou");

        assert_eq!(my_str.as_str(), "Hello ou world ou");
    }

    #[test]
    fn test_replace_to_bigger_size() {
        let mut my_str = ShortString::from_str("Hello my world").unwrap();

        println!("{}", my_str.as_str());

        my_str.replace("my", "beautiful");

        assert_eq!(my_str.as_str(), "Hello beautiful world");
    }

    #[test]
    fn test_replace_to_bigger_size_two_times() {
        let mut my_str = ShortString::from_str("Hello my world my").unwrap();

        println!("{}", my_str.as_str());

        my_str.replace("my", "beautiful");

        assert_eq!(my_str.as_str(), "Hello beautiful world beautiful");
    }

    #[test]
    fn test_replace_to_smaller_size() {
        let mut my_str = ShortString::from_str("Hello beautiful world").unwrap();

        println!("{}", my_str.as_str());

        my_str.replace("beautiful", "my");

        assert_eq!(my_str.as_str(), "Hello my world");
    }

    #[test]
    fn test_replace_to_smaller_size_twice() {
        let mut my_str = ShortString::from_str("Hello beautiful world beautiful").unwrap();

        println!("{}", my_str.as_str());

        my_str.replace("beautiful", "my");

        assert_eq!(my_str.as_str(), "Hello my world my");
    }
}
