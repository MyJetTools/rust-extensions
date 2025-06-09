use std::cell::Cell;

use super::*;

pub struct SliceIterator<'s> {
    slice: &'s [u8],
    pos: Cell<usize>,
}

impl<'s> SliceIterator<'s> {
    pub fn new(slice: &'s [u8]) -> Self {
        Self {
            slice,
            pos: Cell::new(0),
        }
    }

    pub fn from_str(src: &'s str) -> Self {
        Self {
            slice: src.as_bytes(),
            pos: Cell::new(0),
        }
    }
}
impl<'s> ArrayOfBytesIterator for SliceIterator<'s> {
    fn get_src_slice(&self) -> &[u8] {
        self.slice
    }

    fn get_pos(&self) -> usize {
        self.pos.get()
    }

    fn set_pos(&self, pos: usize) {
        self.pos.set(pos);
    }
}
