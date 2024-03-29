use super::*;

pub struct SliceIterator<'s> {
    slice: &'s [u8],
    pos: usize,
}

impl<'s> SliceIterator<'s> {
    pub fn new(slice: &'s [u8]) -> Self {
        Self { slice, pos: 0 }
    }

    pub fn from_str(src: &'s str) -> Self {
        Self {
            slice: src.as_bytes(),
            pos: 0,
        }
    }
}
impl<'s> ArrayOfBytesIterator for SliceIterator<'s> {
    fn get_src_slice(&self) -> &[u8] {
        self.slice
    }

    fn peek_value(&self) -> Option<NextValue> {
        if self.pos < self.slice.len() {
            let result = NextValue {
                pos: self.pos,
                value: self.slice[self.pos],
            };
            Some(result)
        } else {
            None
        }
    }

    fn get_next(&mut self) -> Option<NextValue> {
        if self.pos < self.slice.len() {
            let result = NextValue {
                pos: self.pos,
                value: self.slice[self.pos],
            };
            self.pos += 1;
            Some(result)
        } else {
            None
        }
    }

    fn get_pos(&self) -> usize {
        self.pos
    }

    fn get_slice_to_current_pos(&self, from_pos: usize) -> &[u8] {
        &self.slice[from_pos..self.pos]
    }

    fn get_slice_to_end(&self, from_pos: usize) -> &[u8] {
        &self.slice[from_pos..]
    }

    fn advance(&mut self, amount: usize) -> Option<&[u8]> {
        let pos_after = amount + self.pos;

        if pos_after >= self.slice.len() {
            None
        } else {
            let result = &self.slice[self.pos..pos_after];
            self.pos = pos_after;
            Some(result)
        }
    }

    fn peek_sequence(&self, size: usize, sub_seq: impl Fn(&[u8]) -> bool) -> bool {
        if self.pos + size > self.slice.len() {
            return false;
        }

        sub_seq(&self.slice[self.pos..(self.pos + size)])
    }
}
