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
