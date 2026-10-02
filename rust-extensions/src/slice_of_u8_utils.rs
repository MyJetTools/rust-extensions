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
    // A sequence longer than the source can not be found in it - and without
    // this check the subtraction below would go under zero.
    if sequence.len() > src.len() {
        return None;
    }

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

    #[test]
    fn test_find_sequence_pos_sequence_is_longer_than_src() {
        let src = b"ab";

        // Longer by a single byte...
        assert_eq!(src.find_sequence_pos(b"abc", 0), None);
        // ...and by two and more.
        assert_eq!(src.find_sequence_pos(b"abcd", 0), None);
        assert_eq!(src.find_sequence_pos(b"abcdefgh", 0), None);

        // Wherever the search starts from.
        assert_eq!(src.find_sequence_pos(b"abc", 1), None);
        assert_eq!(src.find_sequence_pos(b"abcd", 2), None);
        assert_eq!(src.find_sequence_pos(b"abcd", 10), None);
    }

    #[test]
    fn test_find_sequence_pos_in_empty_src() {
        let src: &[u8] = b"";

        assert_eq!(src.find_sequence_pos(b"a", 0), None);
        assert_eq!(src.find_sequence_pos(b"ab", 0), None);
        assert_eq!(src.find_sequence_pos(b"ab", 5), None);
    }

    #[test]
    fn test_find_sequence_pos_sequence_of_the_src_size() {
        let src = b"abc";

        assert_eq!(src.find_sequence_pos(b"abc", 0), Some(0));
        assert_eq!(src.find_sequence_pos(b"abd", 0), None);
        assert_eq!(src.find_sequence_pos(b"abc", 1), None);
    }

    #[test]
    fn test_find_sequence_pos_start_is_beyond_the_src() {
        let src = b"1234567890";

        // Right at the end - nothing is left to search in.
        assert_eq!(src.find_sequence_pos(b"0", 10), None);
        // Beyond the end.
        assert_eq!(src.find_sequence_pos(b"0", 11), None);
        assert_eq!(src.find_sequence_pos(b"890", 100), None);
        assert_eq!(src.find_sequence_pos(b"890", usize::MAX), None);

        // The sequence does not fit into what is left after the start.
        assert_eq!(src.find_sequence_pos(b"890", 8), None);
    }
}
