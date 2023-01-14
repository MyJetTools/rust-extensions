pub enum StrOrString<'s> {
    AsStr(&'s str),
    AsString(String),
}

impl<'s> StrOrString<'s> {
    pub fn crate_as_str(s: &'s str) -> Self {
        Self::AsStr(s)
    }
    pub fn crate_as_string(s: String) -> Self {
        Self::AsString(s)
    }

    pub fn as_str(&'s self) -> &'s str {
        match self {
            Self::AsStr(s) => s,
            Self::AsString(s) => s.as_str(),
        }
    }

    pub fn to_string(self) -> String {
        match self {
            Self::AsStr(s) => s.to_string(),
            Self::AsString(s) => s,
        }
    }
}

impl Into<String> for StrOrString<'_> {
    fn into(self) -> String {
        self.to_string()
    }
}

impl ToString for StrOrString<'_> {
    fn to_string(&self) -> String {
        self.as_str().to_string()
    }
}
