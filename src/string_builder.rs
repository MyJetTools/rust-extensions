use std::string::FromUtf8Error;

pub struct StringBuilder {
    buffer: Vec<u8>,
}

impl StringBuilder {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn append_str(&mut self, s: &str) {
        self.buffer.extend(s.as_bytes())
    }

    pub fn append_line(&mut self, s: &str) {
        self.buffer.extend(s.as_bytes());
        self.buffer.push(b'\n');
    }

    pub fn append_bytes(&mut self, s: &[u8]) {
        self.buffer.extend(s);
    }

    pub fn append_byte(&mut self, b: u8) {
        self.buffer.push(b);
    }
    pub fn append_char(&mut self, c: char) {
        self.buffer.push(c as u8)
    }

    pub fn to_string_utf8(self) -> Result<String, FromUtf8Error> {
        return String::from_utf8(self.buffer);
    }
}
