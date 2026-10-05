use std::{
    fmt::{Debug, Display},
    ops::Deref,
};

pub const SHORT_STRING_MAX_LEN: usize = 255;
/// The bytes past the length are always zero: the derived comparisons look at the
/// whole buffer, so a leftover of a longer value would make equal strings unequal.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
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

        let old_len = self.len();
        self.data[1..src.len() + 1].copy_from_slice(src.as_bytes());

        if src.len() < old_len {
            self.data[src.len() + 1..old_len + 1].fill(0);
        }

        self.data[0] = src.len() as u8;
    }

    pub fn insert(&mut self, index: usize, str: &str) -> Result<(), String> {
        let to_insert = str.as_bytes();

        if to_insert.len() + self.len() > SHORT_STRING_MAX_LEN {
            return Err(format!(
                "ShortString is too long. Len must be no more than 255. Now it is {}",
                self.len()
            ));
        }

        if index == self.data.len() {
            self.push_str(str);
            return Ok(());
        }

        let mut result = ShortString::new_empty();
        if index == 0 {
            result.push_str(str);
            result.push_str(self.as_str());
        } else {
            result.push_str(self.as_str()[..index].as_ref());
            result.push_str(str);
            result.push_str(self.as_str()[index..].as_ref());
        }

        self.data.copy_from_slice(result.data.as_ref());

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.data[0] as usize
    }

    pub fn try_push_utf_8_char(&mut self, c: char) -> bool {
        let char_len = c.len_utf8();

        let len = self.len();

        let new_len = len + char_len;

        if new_len > 255 {
            return false;
        }

        let dst = &mut self.data[len + 1..len + char_len + 1];
        c.encode_utf8(dst);

        self.data[0] = new_len as u8;

        true
    }

    pub fn try_push_str(&mut self, src: &str) -> bool {
        let len = self.len();

        let new_len = len + src.len();

        if new_len > 255 {
            return false;
        }

        self.data[len + 1..len + src.len() + 1].copy_from_slice(src.as_bytes());
        self.data[0] = new_len as u8;

        true
    }

    pub fn push_str(&mut self, src: &str) {
        if !self.try_push_str(src) {
            panic!(
                "ShortString is too long. Len must be no more than 255. Before push len is {}. After push len would be {}",
                self.len(),
                self.len()+src.len()
            );
        }
    }

    pub fn try_push(&mut self, c: char) -> bool {
        if c.len_utf8() == 1 {
            let len = self.len();

            let new_len = len + 1;

            if new_len > 255 {
                return false;
            }

            self.data[len + 1] = c as u8;
            self.data[0] = new_len as u8;
            return true;
        }

        self.try_push_utf_8_char(c)
    }

    pub fn push(&mut self, c: char) {
        if !self.try_push(c) {
            panic!(
                "ShortString is too long. Len must be no more than 255. Now it is {}",
                self.len()
            );
        }
    }

    pub fn as_str(&self) -> &str {
        let len = self.len();

        unsafe { std::str::from_utf8_unchecked(&self.data[1..len + 1]) }
    }

    /// Cuts the string to `pos` bytes. Panics when `pos` falls inside a UTF-8 char -
    /// `as_str` would hand out a `str` that is not UTF-8.
    pub fn set_len(&mut self, pos: u8) {
        let old_len = self.len();
        let new_len = pos as usize;

        if new_len < old_len {
            assert!(
                self.as_str().is_char_boundary(new_len),
                "ShortString::set_len({}) cuts the char at this position",
                new_len
            );
            self.data[new_len + 1..old_len + 1].fill(0);
        }

        self.data[0] = pos;
    }

    pub fn compare_with_case_insensitive(&self, other: &str) -> bool {
        crate::str_utils::compare_strings_case_insensitive(self.as_str(), other)
    }

    /// Replaces every occurrence of `from` with `to`, the way `str::replace` does.
    /// Returns `false` and leaves the string as it was when the result would be
    /// longer than 255 bytes.
    pub fn replace(&mut self, from: &str, to: &str) -> bool {
        let mut result = ShortString::new_empty();
        let src = self.as_str();
        let mut copied_up_to = 0;

        for (pos, _) in src.match_indices(from) {
            if !result.try_push_str(&src[copied_up_to..pos]) || !result.try_push_str(to) {
                return false;
            }

            copied_up_to = pos + from.len();
        }

        if !result.try_push_str(&src[copied_up_to..]) {
            return false;
        }

        *self = result;
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

impl AsRef<str> for ShortString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<&str> for ShortString {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<ShortString> for &str {
    fn eq(&self, other: &ShortString) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<&str> for &ShortString {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl crate::AsStr for ShortString {
    fn as_str(&self) -> &str {
        self.as_str()
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

        assert_eq!(my_str, "Hello");
    }

    #[test]
    fn test_replace_the_same_size() {
        let mut my_str = ShortString::from_str("Hello my world my").unwrap();

        my_str.replace("my", "ou");

        assert_eq!(&my_str, "Hello ou world ou");
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

        assert_eq!(&my_str, "Hello my world my");
    }

    /// The search goes on after the inserted text, so a `to` which contains
    /// `from` is not replaced again.
    #[test]
    fn test_replace_with_a_value_containing_the_pattern() {
        let mut my_str = ShortString::from_str("bab").unwrap();

        assert!(my_str.replace("a", "aa"));

        assert_eq!(my_str.as_str(), "baab");
    }

    /// Only the live part is searched - a leftover of a longer value is not.
    #[test]
    fn test_replace_after_update_to_a_shorter_value() {
        let mut my_str = ShortString::from_str("hello world").unwrap();
        my_str.update("hi");

        assert!(my_str.replace("o", "00"));

        assert_eq!(my_str.as_str(), "hi");
    }

    #[test]
    fn test_replace_which_does_not_fit_changes_nothing() {
        let src = "a".repeat(200);
        let mut my_str = ShortString::from_str(src.as_str()).unwrap();

        assert!(!my_str.replace("a", "aa"));

        assert_eq!(my_str.as_str(), src.as_str());
    }

    #[test]
    fn test_equal_after_update_to_a_shorter_value() {
        let mut updated = ShortString::from_str("hello").unwrap();
        updated.update("hi");

        assert!(updated == ShortString::from_str("hi").unwrap());

        let mut cut = ShortString::from_str("hi!").unwrap();
        cut.set_len(2);

        assert!(cut == ShortString::from_str("hi").unwrap());
    }

    #[test]
    #[should_panic(expected = "cuts the char")]
    fn test_set_len_inside_a_char_panics() {
        let mut my_str = ShortString::from_str("Á").unwrap();
        my_str.set_len(1);
    }

    #[test]
    fn test_utf8_two_symbols() {
        let src = 'Á';

        let mut dest = ShortString::new_empty();

        dest.push(src);

        assert_eq!(dest.as_str(), src.to_string());
    }

    #[test]
    fn test_utf8_which_has_two_symbols_char() {
        let src = "AÁB";

        let mut dest = ShortString::new_empty();

        dest.push_str(src);

        assert_eq!(dest.as_str(), src);
    }

    #[test]
    fn test_insert_at_start() {
        let src = "DEFG";

        let mut dest = ShortString::from_str(src).unwrap();
        dest.insert(0, "ABC").unwrap();

        assert_eq!(dest.as_str(), "ABCDEFG");
    }

    #[test]
    fn test_insert_in_the_middle() {
        let src = "ABCG";

        let mut dest = ShortString::from_str(src).unwrap();
        dest.insert(3, "DEF").unwrap();

        assert_eq!(dest.as_str(), "ABCDEFG");
    }

    #[test]
    fn test_insert_in_the_end() {
        let src = "ABC";

        let mut dest = ShortString::from_str(src).unwrap();
        dest.insert(3, "DEFG").unwrap();

        assert_eq!(dest.as_str(), "ABCDEFG");
    }
}
