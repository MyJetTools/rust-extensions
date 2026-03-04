pub trait SliceOfU8Ext {
    fn find_sequence_pos(&self, sequence: &[u8], pos_start: usize) -> Option<usize>;
    fn find_byte_pos(&self, byte: u8, pos_start: usize) -> Option<usize>;
    fn find_pos_by_condition<TCondition: Fn(u8) -> bool>(
        &self,
        pos_start: usize,
        condition: TCondition,
    ) -> Option<usize>;
}

impl<'s> SliceOfU8Ext for &'s [u8] {
    fn find_sequence_pos(&self, sequence: &[u8], pos_start: usize) -> Option<usize> {
        find_sequence_pos(self, sequence, pos_start)
    }

    fn find_byte_pos(&self, byte: u8, pos_start: usize) -> Option<usize> {
        find_byte_pos(self, byte, pos_start)
    }

    fn find_pos_by_condition<TCondition: Fn(u8) -> bool>(
        &self,
        pos_start: usize,
        condition: TCondition,
    ) -> Option<usize> {
        find_pos_by_condition(self, pos_start, condition)
    }
}

impl SliceOfU8Ext for [u8] {
    fn find_sequence_pos(&self, sequence: &[u8], pos_start: usize) -> Option<usize> {
        find_sequence_pos(self, sequence, pos_start)
    }

    fn find_byte_pos(&self, byte: u8, pos_start: usize) -> Option<usize> {
        find_byte_pos(self, byte, pos_start)
    }

    fn find_pos_by_condition<TCondition: Fn(u8) -> bool>(
        &self,
        pos_start: usize,
        condition: TCondition,
    ) -> Option<usize> {
        find_pos_by_condition(self, pos_start, condition)
    }
}

fn find_sequence_pos(src: &[u8], sequence: &[u8], pos_start: usize) -> Option<usize> {
    for i in pos_start..(src.len() - sequence.len() + 1) {
        if &src[i..i + sequence.len()] == sequence {
            return Some(i);
        }
    }
    None
}

fn find_byte_pos(src: &[u8], byte: u8, pos_start: usize) -> Option<usize> {
    for i in pos_start..src.len() {
        if src[i] == byte {
            return Some(i);
        }
    }
    None
}

fn find_pos_by_condition<TCondition: Fn(u8) -> bool>(
    src: &[u8],
    pos_start: usize,
    condition: TCondition,
) -> Option<usize> {
    for pos in pos_start..src.len() {
        if condition(src[pos]) {
            return Some(pos);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::SliceOfU8Ext;

    #[test]
    fn test_find_sequence_pos_in_the_middle() {
        let src = b"1234567890";
        let sequence = b"345";

        let pos = src.find_sequence_pos(sequence, 0);

        assert_eq!(pos, Some(2));

        let pos = src.find_sequence_pos(sequence, 1);

        assert_eq!(pos, Some(2));

        let pos = src.find_sequence_pos(sequence, 2);

        assert_eq!(pos, Some(2));

        let pos = src.find_sequence_pos(sequence, 3);

        assert!(pos.is_none());
    }

    #[test]
    fn test_find_sequence_pos_at_start() {
        let src = b"1234567890";
        let sequence = b"123";

        let pos = src.find_sequence_pos(sequence, 0);

        assert_eq!(pos, Some(0));

        let pos = src.find_sequence_pos(sequence, 1);

        assert!(pos.is_none());
    }

    #[test]
    fn test_find_sequence_pos_at_end() {
        let src = b"1234567890";
        let sequence = b"890";

        let pos = src.find_sequence_pos(sequence, 7);

        assert_eq!(pos, Some(7));
    }
}
