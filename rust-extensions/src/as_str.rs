pub trait AsStr {
    fn as_str(&self) -> &str;
}

impl AsStr for String {
    fn as_str(&self) -> &str {
        self.as_str()
    }
}

impl AsStr for &'_ str {
    fn as_str(&self) -> &str {
        self
    }
}

impl AsStr for str {
    fn as_str(&self) -> &str {
        self
    }
}

impl AsStr for &'_ String {
    fn as_str(&self) -> &str {
        self
    }
}
