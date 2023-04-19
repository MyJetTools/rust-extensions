pub trait AsU8Slice {
    fn as_slice(&self) -> &[u8];
}

impl AsU8Slice for String {
    fn as_slice(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<'s> AsU8Slice for &'s str {
    fn as_slice(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<'s> AsU8Slice for &'s [u8] {
    fn as_slice(&self) -> &[u8] {
        self
    }
}

impl AsU8Slice for Vec<u8> {
    fn as_slice(&self) -> &[u8] {
        self
    }
}
