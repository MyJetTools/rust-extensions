pub struct StringBuilder {
    buffer: String,
}

impl StringBuilder {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    pub fn append_str(&mut self, s: &str) {
        self.buffer.push_str(s)
    }

    pub fn append_line(&mut self, s: &str) {
        self.buffer.push_str(s);
        self.buffer.push('\n');
    }

    pub fn append_bytes(&mut self, s: &[u8]) -> Result<(), std::str::Utf8Error> {
        let str = std::str::from_utf8(s)?;
        self.buffer.push_str(str);
        Ok(())
    }

    pub fn append_byte(&mut self, b: u8) {
        self.buffer.push(b as char);
    }
    pub fn append_char(&mut self, c: char) {
        self.buffer.push(c)
    }

    pub fn to_string_utf8(self) -> String {
        self.buffer
    }
}
