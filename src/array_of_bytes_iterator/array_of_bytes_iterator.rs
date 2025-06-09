pub struct NextValue {
    pub value: u8,
    pub pos: usize,
}

pub trait ArrayOfBytesIterator {
    fn get_pos(&self) -> usize;
    fn set_pos(&self, pos: usize);

    fn get_src_slice(&self) -> &[u8];

    fn peek_value(&self) -> Option<NextValue> {
        let pos = self.get_pos();
        let slice = self.get_src_slice();
        if pos < slice.len() {
            let result = NextValue {
                pos: pos,
                value: slice[pos],
            };
            Some(result)
        } else {
            None
        }
    }

    fn get_slice_to_current_pos(&self, from_pos: usize) -> &[u8] {
        let pos = self.get_pos();

        let slice = self.get_src_slice();
        &slice[from_pos..pos]
    }

    fn get_slice_to_end(&self, from_pos: usize) -> &[u8] {
        let slice = self.get_src_slice();
        &slice[from_pos..]
    }

    fn get_next(&self) -> Option<NextValue> {
        let pos = self.get_pos();
        let slice = self.get_src_slice();
        if pos < slice.len() {
            let result = NextValue {
                pos,
                value: slice[pos],
            };
            self.set_pos(pos + 1);
            Some(result)
        } else {
            None
        }
    }

    fn peek_sequence(&self, size: usize, sub_seq: impl Fn(&[u8]) -> bool) -> bool {
        let pos = self.get_pos();
        let slice = self.get_src_slice();
        if pos + size > slice.len() {
            return false;
        }

        sub_seq(&slice[pos..(pos + size)])
    }

    fn peek_and_find_sequence_pos(&self, sequence_to_fine: &[u8]) -> Option<usize> {
        let mut pos = self.get_pos();

        let slice = self.get_src_slice();

        while pos + sequence_to_fine.len() < slice.len() {
            let pos_slice = &slice[pos..pos + sequence_to_fine.len()];

            if pos_slice == sequence_to_fine {
                return Some(pos);
            }

            pos += 1;
        }

        None
    }

    fn advance(&self, amount: usize) -> Option<&[u8]> {
        let pos = self.get_pos();
        let pos_after = amount + pos;

        let data = self.get_src_slice();

        if pos_after >= data.len() {
            None
        } else {
            let result = &data[pos..pos_after];
            self.set_pos(pos_after);
            Some(result)
        }
    }
}
