use std::fmt::Debug;

use crate::{ShortString, SliceOrVec};

#[derive(Debug, Clone)]
pub struct StrOrString<'s> {
    data: StrOrStringData<'s>,
    from: Option<usize>,
    to: Option<usize>,
}

impl<'s> StrOrString<'s> {
    pub fn create_as_str(s: &'s str) -> Self {
        Self {
            data: StrOrStringData::AsStr(s),
            from: None,
            to: None,
        }
    }

    pub fn create_as_string(s: String) -> Self {
        Self {
            data: StrOrStringData::AsString(s),
            from: None,
            to: None,
        }
    }
    /*
    pub fn create_as_short_string(s: ShortString) -> Self {
        Self {
            data: StrOrStringData::AsShortString(s),
            from: None,
            to: None,
        }
    }


    pub fn create_as_short_string_or_string(s: &str) -> Self {
        let data = if let Some(s) = ShortString::from_str(s) {
            StrOrStringData::AsShortString(s)
        } else {
            StrOrStringData::AsString(s.to_string())
        };

        Self {
            data,
            from: None,
            to: None,
        }
    }
     */
    pub fn slice_it(&mut self, from: Option<usize>, to: Option<usize>) {
        self.from = from;
        self.to = to;
    }

    fn has_data_to_cut(&self) -> bool {
        self.from.is_some() || self.to.is_some()
    }

    pub fn as_str(&'s self) -> &'s str {
        let result = match &self.data {
            StrOrStringData::AsStr(s) => s,
            StrOrStringData::AsString(s) => s.as_str(),
        };

        cut_data(result, self.from, self.to)
    }

    pub fn to_string(&self) -> String {
        let has_data_to_cut = self.has_data_to_cut();
        match &self.data {
            StrOrStringData::AsStr(s) => cut_data(s, self.from, self.to).to_string(),
            StrOrStringData::AsString(s) => {
                if has_data_to_cut {
                    cut_data(&s, self.from, self.to).to_string()
                } else {
                    s.to_string()
                }
            }
        }
    }

    pub fn into_string(self) -> String {
        let has_data_to_cut = self.has_data_to_cut();
        match self.data {
            StrOrStringData::AsStr(s) => cut_data(s, self.from, self.to).to_string(),
            StrOrStringData::AsString(s) => {
                if has_data_to_cut {
                    cut_data(&s, self.from, self.to).to_string()
                } else {
                    s
                }
            }
        }
    }

    pub fn to_short_string(&self) -> ShortString {
        let has_data_to_cut = self.has_data_to_cut();

        if has_data_to_cut {
            match &self.data {
                StrOrStringData::AsStr(s) => {
                    ShortString::from_str(cut_data(s, self.from, self.to)).unwrap()
                }
                StrOrStringData::AsString(s) => {
                    ShortString::from_str(cut_data(&s, self.from, self.to)).unwrap()
                }
            }
        } else {
            match &self.data {
                StrOrStringData::AsStr(s) => ShortString::from_str(s).unwrap(),
                StrOrStringData::AsString(s) => ShortString::from_str(&s).unwrap(),
            }
        }
    }

    pub fn into_short_string(self) -> ShortString {
        let has_data_to_cut = self.has_data_to_cut();

        if has_data_to_cut {
            match self.data {
                StrOrStringData::AsStr(s) => {
                    ShortString::from_str(cut_data(s, self.from, self.to)).unwrap()
                }
                StrOrStringData::AsString(s) => {
                    ShortString::from_str(cut_data(&s, self.from, self.to)).unwrap()
                }
            }
        } else {
            match self.data {
                StrOrStringData::AsStr(s) => ShortString::from_str(s).unwrap(),
                StrOrStringData::AsString(s) => ShortString::from_str(&s).unwrap(),
            }
        }
    }

    /*
    pub fn from_str_convert_to_lower_case(src: &str) -> Self {
        let data = if let Some(s) = ShortString::from_str_convert_to_lower_case(src) {
            StrOrStringData::AsShortString(s)
        } else {
            StrOrStringData::AsString(src.to_string())
        };

        Self {
            data,
            from: None,
            to: None,
        }
    }
     */

    pub fn compare_with_case_insensitive(&self, other: &str) -> bool {
        crate::str_utils::compare_strings_case_insensitive(self.as_str(), other)
    }
}

impl std::fmt::Display for StrOrString<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

fn cut_data(src: &str, src_from: Option<usize>, src_to: Option<usize>) -> &str {
    if let Some(from) = src_from {
        if let Some(to) = src_to {
            return &src[from..to];
        } else {
            return &src[from..];
        }
    }

    if let Some(to) = src_to {
        return &src[..to];
    }
    src
}

impl<'s> Into<StrOrString<'s>> for &'s str {
    fn into(self) -> StrOrString<'s> {
        StrOrString::create_as_str(self)
    }
}

impl<'s> Into<StrOrString<'s>> for &'s String {
    fn into(self) -> StrOrString<'s> {
        StrOrString::create_as_str(self)
    }
}

impl<'s> Into<StrOrString<'s>> for String {
    fn into(self) -> StrOrString<'s> {
        StrOrString::create_as_string(self)
    }
}

impl<'s> Into<StrOrString<'s>> for SliceOrVec<'s, u8> {
    fn into(self) -> StrOrString<'s> {
        match self {
            SliceOrVec::AsSlice(slice) => {
                StrOrString::create_as_str(std::str::from_utf8(slice).unwrap())
            }
            SliceOrVec::AsVec(vec) => {
                StrOrString::create_as_string(String::from_utf8(vec).unwrap())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum StrOrStringData<'s> {
    AsStr(&'s str),
    AsString(String),
}

impl Into<String> for StrOrString<'_> {
    fn into(self) -> String {
        self.into_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::StrOrString;

    #[test]
    fn test_src_with_cut() {
        let mut src = StrOrString::create_as_str("123");
        src.slice_it(1.into(), 2.into());

        assert_eq!("2", src.as_str())
    }

    #[test]
    fn test_string_with_cut() {
        let mut src = StrOrString::create_as_string("123".to_string());
        src.slice_it(1.into(), 2.into());

        assert_eq!("2", src.as_str())
    }
    #[test]
    fn test_src_with_no_cut() {
        let src = StrOrString::create_as_str("123");

        assert_eq!("123", src.as_str())
    }

    #[test]
    fn test_string_with_no_cut() {
        let src = StrOrString::create_as_string("123".to_string());

        assert_eq!("123", src.as_str());
        println!("{}", src);
    }
}
