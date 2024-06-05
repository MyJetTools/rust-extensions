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
                if !value.try_push_str(c) {
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
            MaybeShortString::AsShortString(value) => StrOrString::create_as_short_string(value),
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
