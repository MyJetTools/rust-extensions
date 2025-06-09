use std::cell::Cell;

use super::*;

pub struct VecIterator {
    data: Vec<u8>,
    pos: Cell<usize>,
}

impl Default for VecIterator {
    fn default() -> Self {
        Self {
            data: vec![],
            pos: 0.into(),
        }
    }
}

impl VecIterator {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            pos: 0.into(),
        }
    }

    pub fn from_str(src: &str) -> Self {
        Self {
            data: src.as_bytes().to_vec(),
            pos: 0.into(),
        }
    }

    pub fn extend(&mut self, slice: &[u8]) {
        self.data.extend_from_slice(slice);
    }

    pub fn gc(&mut self) {
        let pos = self.pos.get();

        if pos > 0 {
            self.data.drain(..pos);
            self.pos.set(0);
        }
    }
}
impl ArrayOfBytesIterator for VecIterator {
    fn get_src_slice(&self) -> &[u8] {
        &self.data
    }

    fn get_pos(&self) -> usize {
        self.pos.get()
    }

    fn set_pos(&self, pos: usize) {
        self.pos.set(pos);
    }
}
