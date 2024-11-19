use crate::UnsafeValue;

use super::*;

pub struct SliceIterator<'s> {
    slice: &'s [u8],
    pos: UnsafeValue<usize>,
}

impl<'s> SliceIterator<'s> {
    pub fn new(slice: &'s [u8]) -> Self {
        Self {
            slice,
            pos: UnsafeValue::new(0),
        }
    }

    pub fn from_str(src: &'s str) -> Self {
        Self {
            slice: src.as_bytes(),
            pos: UnsafeValue::new(0),
        }
    }
}
impl<'s> ArrayOfBytesIterator for SliceIterator<'s> {
    fn get_src_slice(&self) -> &[u8] {
        self.slice
    }

    fn peek_value(&self) -> Option<NextValue> {
        let pos = self.pos.get_value();
        if pos < self.slice.len() {
            let result = NextValue {
                pos: pos,
                value: self.slice[pos],
            };
            Some(result)
        } else {
            None
        }
    }

    fn get_next(&self) -> Option<NextValue> {
        let pos = self.pos.get_value();
        if pos < self.slice.len() {
            let result = NextValue {
                pos,
                value: self.slice[pos],
            };
            self.pos.set_value(pos + 1);
            Some(result)
        } else {
            None
        }
    }

    fn get_pos(&self) -> usize {
        self.pos.get_value()
    }

    fn get_slice_to_current_pos(&self, from_pos: usize) -> &[u8] {
        let pos = self.pos.get_value();
        &self.slice[from_pos..pos]
    }

    fn get_slice_to_end(&self, from_pos: usize) -> &[u8] {
        &self.slice[from_pos..]
    }

    fn advance(&self, amount: usize) -> Option<&[u8]> {
        let pos = self.pos.get_value();
        let pos_after = amount + pos;

        if pos_after >= self.slice.len() {
            None
        } else {
            let result = &self.slice[pos..pos_after];
            self.pos.set_value(pos_after);
            Some(result)
        }
    }

    fn peek_sequence(&self, size: usize, sub_seq: impl Fn(&[u8]) -> bool) -> bool {
        let pos = self.pos.get_value();
        if pos + size > self.slice.len() {
            return false;
        }

        sub_seq(&self.slice[pos..(pos + size)])
    }
}
