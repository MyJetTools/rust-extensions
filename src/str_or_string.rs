pub struct StrOrString<'s> {
    as_str: Option<&'s str>,
    as_string: Option<String>,
}

impl<'s> StrOrString<'s> {
    pub fn crate_as_str(s: &'s str) -> Self {
        Self {
            as_str: Some(s),
            as_string: None,
        }
    }
    pub fn crate_as_string(s: String) -> Self {
        Self {
            as_str: None,
            as_string: Some(s),
        }
    }

    pub fn as_str(&'s self) -> &'s str {
        if let Some(as_str) = self.as_str {
            return as_str;
        }

        if let Some(as_str) = self.as_string.as_ref() {
            return as_str;
        }

        panic!("Somehow we are here");
    }

    pub fn to_string(self) -> String {
        if let Some(as_str) = self.as_str {
            return as_str.to_string();
        }

        if let Some(as_string) = self.as_string {
            return as_string;
        }

        panic!("Somehow we are here");
    }
}
