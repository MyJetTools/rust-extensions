use super::NextValue;

#[async_trait::async_trait]
pub trait ArrayOfBytesIteratorAsync {
    fn peek_value(&self) -> Option<NextValue>;
    async fn get_next(&mut self) -> std::io::Result<Option<NextValue>>;
    fn get_pos(&self) -> usize;

    async fn get_slice_to_current_pos(&self, from_pos: usize) -> std::io::Result<Vec<u8>>;
    async fn get_slice_to_end(&self, from_pos: usize) -> std::io::Result<Vec<u8>>;

    async fn advance(&mut self, amount: usize) -> std::io::Result<Option<Vec<u8>>>;
}
