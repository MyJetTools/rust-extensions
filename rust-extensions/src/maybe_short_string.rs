use std::fmt::{Debug, Display};

use crate::{ShortString, StrOrString};

pub enum MaybeShortString {
    AsShortString(ShortString),
    AsString(String),
}

impl MaybeShortString {
    pub fn new() -> Self {
        MaybeShortString::AsShortString(ShortString::new_empty())
    }

    pub fn from_str(value: &str) -> Self {
        if value.as_bytes().len() <= crate::SHORT_STRING_MAX_LEN {
            MaybeShortString::AsShortString(ShortString::from_str(value).unwrap())
        } else {
            MaybeShortString::AsString(value.to_string())
        }
    }

    pub fn from_str_as_lower_case(src: &str) -> Self {
        let mut result = Self::new();

        for c in src.chars() {
            result.push(c.to_ascii_lowercase());
        }

        result
    }

    pub fn from_str_as_upper_case(src: &str) -> Self {
        let mut result = Self::new();

        for c in src.chars() {
            result.push(c.to_ascii_uppercase());
        }

        result
    }

    pub fn push(&mut self, c: char) {
        match self {
            MaybeShortString::AsShortString(value) => {
                if value.try_push(c) {
                    return;
                }

                let mut new_value = String::new();
                new_value.push_str(value.as_str());
                new_value.push(c);
                *self = MaybeShortString::AsString(new_value);
            }
            MaybeShortString::AsString(value) => value.push(c),
        }
    }

    pub fn push_str(&mut self, c: &str) {
        match self {
            MaybeShortString::AsShortString(value) => {
                if value.try_push_str(c) {
                    return;
                }

                let mut new_value = String::new();
                new_value.push_str(value.as_str());
                new_value.push_str(c);
                *self = MaybeShortString::AsString(new_value);
            }
            MaybeShortString::AsString(value) => value.push_str(c),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            MaybeShortString::AsShortString(value) => value.len(),
            MaybeShortString::AsString(value) => value.len(),
        }
    }

    pub fn into<'s>(self) -> StrOrString<'s> {
        match self {
            MaybeShortString::AsShortString(value) => {
                StrOrString::create_as_string(value.to_string())
            }
            MaybeShortString::AsString(value) => StrOrString::create_as_string(value),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            MaybeShortString::AsShortString(value) => value.as_str(),
            MaybeShortString::AsString(value) => value.as_str(),
        }
    }
}

impl Display for MaybeShortString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Debug for MaybeShortString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AsShortString(arg0) => f.debug_tuple("AsShortString").field(arg0).finish(),
            Self::AsString(arg0) => f.debug_tuple("AsString").field(arg0).finish(),
        }
    }
}

impl Into<MaybeShortString> for String {
    fn into(self) -> MaybeShortString {
        MaybeShortString::AsString(self)
    }
}

impl Into<MaybeShortString> for ShortString {
    fn into(self) -> MaybeShortString {
        MaybeShortString::AsShortString(self)
    }
}

impl<'s> Into<MaybeShortString> for &'s str {
    fn into(self) -> MaybeShortString {
        MaybeShortString::from_str(self)
    }
}

impl<'s> Into<MaybeShortString> for &'s String {
    fn into(self) -> MaybeShortString {
        MaybeShortString::from_str(self)
    }
}

impl Into<String> for MaybeShortString {
    fn into(self) -> String {
        match self {
            MaybeShortString::AsShortString(value) => value.to_string(),
            MaybeShortString::AsString(value) => value,
        }
    }
}

impl TryInto<ShortString> for MaybeShortString {
    type Error = String;

    fn try_into(self) -> Result<ShortString, Self::Error> {
        match self {
            MaybeShortString::AsShortString(value) => Ok(value),
            MaybeShortString::AsString(value) => Err(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MaybeShortString;

    #[test]
    fn push_str_keeps_a_short_value_short() {
        let mut value = MaybeShortString::from_str("x");
        value.push_str("ab");

        assert_eq!(value.as_str(), "xab");
        assert!(matches!(value, MaybeShortString::AsShortString(_)));
    }

    #[test]
    fn push_str_which_does_not_fit_promotes_to_string() {
        let long = "y".repeat(300);

        let mut value = MaybeShortString::from_str("x");
        value.push_str(long.as_str());

        assert_eq!(value.len(), 301);
        assert!(value.as_str().starts_with("xy"));
        assert!(matches!(value, MaybeShortString::AsString(_)));
    }
}
