pub fn find_sequence_pos(src: &[u8], sequence: &[u8], pos_start: usize) -> Option<usize> {
    for i in pos_start..(src.len() - sequence.len() + 1) {
        if &src[i..i + sequence.len()] == sequence {
            return Some(i);
        }
    }
    None
}

pub fn find_byte_pos(src: &[u8], byte: u8, pos_start: usize) -> Option<usize> {
    for i in pos_start..src.len() {
        if src[i] == byte {
            return Some(i);
        }
    }
    None
}

pub fn find_pos_by_condition<TCondition: Fn(u8) -> bool>(
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

    #[test]
    fn test_find_sequence_pos_in_the_middle() {
        let src = b"1234567890";
        let sequence = b"345";

        let pos = super::find_sequence_pos(src, sequence, 0);

        assert_eq!(pos, Some(2));

        let pos = super::find_sequence_pos(src, sequence, 1);

        assert_eq!(pos, Some(2));

        let pos = super::find_sequence_pos(src, sequence, 2);

        assert_eq!(pos, Some(2));

        let pos = super::find_sequence_pos(src, sequence, 3);

        assert!(pos.is_none());
    }

    #[test]
    fn test_find_sequence_pos_at_start() {
        let src = b"1234567890";
        let sequence = b"123";

        let pos = super::find_sequence_pos(src, sequence, 0);

        assert_eq!(pos, Some(0));

        let pos = super::find_sequence_pos(src, sequence, 1);

        assert!(pos.is_none());
    }

    #[test]
    fn test_find_sequence_pos_at_end() {
        let src = b"1234567890";
        let sequence = b"890";

        let pos = super::find_sequence_pos(src, sequence, 7);

        assert_eq!(pos, Some(7));
    }
}
