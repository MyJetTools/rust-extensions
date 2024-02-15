pub struct NextValue {
    pub value: u8,
    pub pos: usize,
}

pub trait ArrayOfBytesIterator<'s> {
    fn peek_value(&self) -> Option<NextValue>;
    fn get_next(&mut self) -> Option<NextValue>;
    fn get_pos(&self) -> usize;

    fn get_slice_to_current_pos(&self, from_pos: usize) -> &[u8];
    fn get_slice_to_end(&self, from_pos: usize) -> &[u8];

    fn advance(&mut self, amount: usize) -> Option<&[u8]>;

    fn get_src_slice(&self) -> &[u8];

    fn peek_sequence(&'s self, size: usize, sub_seq: impl Fn(&'s [u8]) -> bool) -> bool;
}
