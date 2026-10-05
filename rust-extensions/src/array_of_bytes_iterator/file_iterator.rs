use tokio::{
    io::{AsyncReadExt, AsyncSeekExt},
    sync::Mutex,
};

use super::{ArrayOfBytesIteratorAsync, NextValue};
pub struct Buffer {
    pub buffer: Vec<u8>,
    pub offset: usize,
    pub buffer_size: usize,
}

impl Buffer {
    pub fn get_byte(&self, file_offset: usize) -> u8 {
        let pos = file_offset - self.offset;
        self.buffer[pos]
    }

    pub fn beyond_buffer(&self, file_offset: usize) -> bool {
        file_offset >= self.offset + self.buffer_size
    }

    /// Whether the file range `file_from..file_to` is in the buffer.
    pub fn contains(&self, file_from: usize, file_to: usize) -> bool {
        file_from >= self.offset && file_to <= self.offset + self.buffer_size
    }

    pub fn get_slice(&self, file_from: usize, file_to: usize) -> &[u8] {
        let from = file_from - self.offset;
        let to = file_to - self.offset;
        &self.buffer[from..to]
    }
}

/// Reads a file through a window of `buffer_size` bytes, with the same semantics as
/// the in-memory iterators. While the position is inside the file, the window
/// holds it - so `peek_value` needs no I/O.
pub struct FileIterator {
    pos: usize,
    file: Mutex<tokio::fs::File>,
    buffer: Buffer,
    file_size: usize,
}

impl FileIterator {
    pub async fn new(file_name: &str, buffer_size: usize) -> std::io::Result<Self> {
        let file_size = tokio::fs::metadata(file_name).await?.len() as usize;
        let file = tokio::fs::File::open(file_name).await?;

        let mut result = Self {
            pos: 0,
            file: Mutex::new(file),
            buffer: Buffer {
                buffer: vec![0u8; buffer_size],
                offset: 0,
                buffer_size,
            },
            file_size,
        };

        result.load_buffer_at(0).await?;

        Ok(result)
    }

    async fn load_slice_from_file(
        &self,
        from_pos: usize,
        to_pos: usize,
    ) -> std::io::Result<Vec<u8>> {
        let mut file = self.file.lock().await;
        file.seek(std::io::SeekFrom::Start(from_pos as u64)).await?;

        let size_to_load = to_pos - from_pos;
        let mut result = vec![0u8; size_to_load];

        file.read_exact(&mut result).await?;

        Ok(result)
    }

    async fn read_range(&self, from_pos: usize, to_pos: usize) -> std::io::Result<Vec<u8>> {
        if self.buffer.contains(from_pos, to_pos) {
            return Ok(self.buffer.get_slice(from_pos, to_pos).to_vec());
        }

        self.load_slice_from_file(from_pos, to_pos).await
    }

    async fn load_buffer_at(&mut self, offset: usize) -> std::io::Result<()> {
        let size_to_load = self.file_size.saturating_sub(offset).min(self.buffer.buffer_size);

        let mut file = self.file.lock().await;
        file.seek(std::io::SeekFrom::Start(offset as u64)).await?;
        file.read_exact(&mut self.buffer.buffer[..size_to_load])
            .await?;

        self.buffer.offset = offset;

        Ok(())
    }

    /// Moves the window to the position once the position has left it.
    async fn follow_pos(&mut self) -> std::io::Result<()> {
        if self.pos < self.file_size && !self.buffer.contains(self.pos, self.pos + 1) {
            self.load_buffer_at(self.pos).await?;
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl ArrayOfBytesIteratorAsync for FileIterator {
    fn peek_value(&self) -> Option<NextValue> {
        if self.pos >= self.file_size {
            return None;
        }

        Some(NextValue {
            value: self.buffer.get_byte(self.pos),
            pos: self.pos,
        })
    }

    async fn get_next(&mut self) -> std::io::Result<Option<NextValue>> {
        let Some(result) = self.peek_value() else {
            return Ok(None);
        };

        self.pos += 1;
        self.follow_pos().await?;

        Ok(Some(result))
    }

    fn get_pos(&self) -> usize {
        self.pos
    }

    async fn get_slice_to_current_pos(&self, from_pos: usize) -> std::io::Result<Vec<u8>> {
        self.read_range(from_pos, self.pos).await
    }

    async fn get_slice_to_end(&self, from_pos: usize) -> std::io::Result<Vec<u8>> {
        self.read_range(from_pos, self.file_size).await
    }

    async fn advance(&mut self, amount: usize) -> std::io::Result<Option<Vec<u8>>> {
        let end_pos = self.pos + amount;

        if end_pos > self.file_size {
            return Ok(None);
        }

        let result = self.read_range(self.pos, end_pos).await?;

        self.pos = end_pos;
        self.follow_pos().await?;

        Ok(Some(result))
    }
}

#[cfg(test)]
mod tests {
    use super::{ArrayOfBytesIteratorAsync, FileIterator};

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
    }

    /// A file of `content` in the temp dir, removed on drop.
    struct TempFile(std::path::PathBuf);

    impl TempFile {
        fn new(name: &str, content: &[u8]) -> Self {
            let path = std::env::temp_dir().join(format!(
                "rust-extensions-file-iterator-{}-{}",
                std::process::id(),
                name
            ));
            std::fs::write(&path, content).unwrap();
            Self(path)
        }

        fn path(&self) -> &str {
            self.0.to_str().unwrap()
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn get_next_walks_through_buffer_windows_and_stops_at_the_end() {
        let file = TempFile::new("walk", b"0123456789");

        rt().block_on(async {
            // a window of 4 bytes: the 10 bytes take three windows
            let mut src = FileIterator::new(file.path(), 4).await.unwrap();

            let mut read = Vec::new();
            while let Some(next) = src.get_next().await.unwrap() {
                assert_eq!(next.pos, read.len());
                read.push(next.value);
            }

            assert_eq!(read, b"0123456789");
            assert!(src.peek_value().is_none());
        });
    }

    #[test]
    fn advance_moves_the_position_inside_and_across_windows() {
        let file = TempFile::new("advance", b"0123456789");

        rt().block_on(async {
            let mut src = FileIterator::new(file.path(), 4).await.unwrap();

            assert_eq!(src.advance(2).await.unwrap(), Some(b"01".to_vec()));
            assert_eq!(src.get_pos(), 2);
            assert_eq!(src.peek_value().unwrap().value, b'2');

            // crosses into the next windows
            assert_eq!(src.advance(5).await.unwrap(), Some(b"23456".to_vec()));
            assert_eq!(src.peek_value().unwrap().value, b'7');

            assert_eq!(src.advance(4).await.unwrap(), None);
            assert_eq!(src.advance(3).await.unwrap(), Some(b"789".to_vec()));
            assert!(src.get_next().await.unwrap().is_none());
        });
    }

    #[test]
    fn slices_by_position() {
        let file = TempFile::new("slices", b"0123456789");

        rt().block_on(async {
            let mut src = FileIterator::new(file.path(), 4).await.unwrap();
            src.advance(6).await.unwrap();

            assert_eq!(src.get_slice_to_current_pos(1).await.unwrap(), b"12345");
            assert_eq!(src.get_slice_to_end(7).await.unwrap(), b"789");
        });
    }

    #[test]
    fn a_file_smaller_than_the_buffer() {
        let file = TempFile::new("small", b"ab");

        rt().block_on(async {
            let mut src = FileIterator::new(file.path(), 1024).await.unwrap();

            assert_eq!(src.get_next().await.unwrap().unwrap().value, b'a');
            assert_eq!(src.get_next().await.unwrap().unwrap().value, b'b');
            assert!(src.get_next().await.unwrap().is_none());
        });
    }
}
