use std::collections::HashMap;

use crate::StrOrString;

pub struct LogEventContextBuilder(Option<HashMap<String, String>>);

impl LogEventContextBuilder {
    pub fn new() -> Self {
        Self(None)
    }

    pub fn add_element(
        mut self,
        key: impl Into<StrOrString<'static>>,
        value: impl Into<StrOrString<'static>>,
    ) -> Self {
        if self.0.is_none() {
            self.0 = Some(HashMap::new());
        }

        self.0
            .as_mut()
            .unwrap()
            .insert(key.into().to_string(), format_value(value.into()));
        self
    }

    pub fn get_result(self) -> Option<HashMap<String, String>> {
        self.0
    }
}

fn format_value(src: StrOrString) -> String {
    let src_as_bytes = src.as_str().as_bytes();

    let mut has_data_to_escape = false;
    for b in src_as_bytes {
        if *b < 32u8 {
            has_data_to_escape = true;
            break;
        }
    }

    if !has_data_to_escape {
        return src.to_string();
    }

    let mut result = String::with_capacity(src_as_bytes.len());

    for b in src_as_bytes {
        if *b >= 32 {
            result.push(*b as char);
        }
    }

    result
}

impl Into<LogEventContextBuilder> for Option<LogEventContextBuilder> {
    fn into(self) -> LogEventContextBuilder {
        match self {
            Some(src) => src,
            None => LogEventContextBuilder::new(),
        }
    }
}
