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

    fn peek_value(&self) -> Option<NextValue> {
        let pos = self.pos.get();
        if pos < self.data.len() {
            let result = NextValue {
                pos: pos,
                value: self.data[pos],
            };
            Some(result)
        } else {
            None
        }
    }

    fn get_next(&self) -> Option<NextValue> {
        let pos = self.pos.get();
        if pos < self.data.len() {
            let result = NextValue {
                pos,
                value: self.data[pos],
            };
            self.pos.set(pos + 1);
            Some(result)
        } else {
            None
        }
    }

    fn get_pos(&self) -> usize {
        self.pos.get()
    }

    fn get_slice_to_current_pos(&self, from_pos: usize) -> &[u8] {
        let pos = self.pos.get();
        &self.data[from_pos..pos]
    }

    fn get_slice_to_end(&self, from_pos: usize) -> &[u8] {
        &self.data[from_pos..]
    }

    fn advance(&self, amount: usize) -> Option<&[u8]> {
        let pos = self.pos.get();
        let pos_after = amount + pos;

        if pos_after >= self.data.len() {
            None
        } else {
            let result = &self.data[pos..pos_after];
            self.pos.set(pos_after);
            Some(result)
        }
    }

    fn peek_sequence(&self, size: usize, sub_seq: impl Fn(&[u8]) -> bool) -> bool {
        let pos = self.pos.get();
        if pos + size > self.data.len() {
            return false;
        }

        sub_seq(&self.data[pos..(pos + size)])
    }
}
