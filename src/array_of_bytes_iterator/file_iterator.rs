use tokio::{
    io::{AsyncReadExt, AsyncSeekExt},
    sync::Mutex,
};

use super::{ArrayOfBytesIteratorAsync, NextValue};
const BUFFER_SIZE: usize = 1024;
pub struct Buffer {
    pub buffer: [u8; BUFFER_SIZE],
    pub offset: usize,
}

impl Buffer {
    pub fn get_byte(&self, file_offset: usize) -> u8 {
        let pos = file_offset - self.offset;
        self.buffer[pos]
    }

    pub fn beyond_buffer(&self, file_offset: usize) -> bool {
        file_offset >= self.offset + BUFFER_SIZE
    }

    pub fn get_slice(&self, file_from: usize, file_to: usize) -> &[u8] {
        let from = file_from - self.offset;
        let to = file_to - self.offset;
        &self.buffer[from..to]
    }
}

pub struct JsonFileIterator {
    pos: usize,
    file: Mutex<tokio::fs::File>,
    buffer: Buffer,
    file_size: usize,
}

impl JsonFileIterator {
    pub async fn new(file_name: &str) -> std::io::Result<Self> {
        let mut buffer = Buffer {
            buffer: [0; BUFFER_SIZE],
            offset: 0,
        };
        let file_size = tokio::fs::metadata(file_name).await?.len() as usize;

        let mut file = tokio::fs::File::open(file_name).await?;
        if file_size <= BUFFER_SIZE {
            file.read_exact(&mut buffer.buffer[..file_size]).await?;
        } else {
            file.read_exact(&mut buffer.buffer).await?;
        }

        Ok(Self {
            pos: 0,
            file: Mutex::new(file),
            buffer,
            file_size,
        })
    }

    async fn load_slice_from_file(
        &self,
        from_pos: usize,
        to_pos: usize,
    ) -> std::io::Result<Vec<u8>> {
        let mut file = self.file.lock().await;
        file.seek(std::io::SeekFrom::Start(from_pos as u64)).await?;

        let size_to_load = to_pos - from_pos;
        let mut result = Vec::with_capacity(size_to_load);

        unsafe { result.set_len(size_to_load) }

        file.read_exact(&mut result).await?;

        Ok(result)
    }

    async fn load_next_chunk(&mut self) -> std::io::Result<()> {
        let next_offset = self.buffer.offset + BUFFER_SIZE;

        let remaining_to_load = self.file_size - next_offset;

        if remaining_to_load == 0 {
            return Ok(());
        }

        let mut file = self.file.lock().await;
        file.seek(std::io::SeekFrom::Start(next_offset as u64))
            .await?;

        if remaining_to_load <= BUFFER_SIZE {
            file.read_exact(&mut self.buffer.buffer[..remaining_to_load])
                .await?;
        } else {
            file.read_exact(&mut self.buffer.buffer).await?;
        }

        self.buffer.offset = next_offset;

        Ok(())
    }
}

#[async_trait::async_trait]
impl ArrayOfBytesIteratorAsync for JsonFileIterator {
    fn peek_value(&self) -> Option<NextValue> {
        if self.pos >= self.file_size {
            return None;
        }
        let value = self.buffer.get_byte(self.pos);

        Some(NextValue {
            value,
            pos: self.pos,
        })
    }

    async fn get_next(&mut self) -> std::io::Result<Option<NextValue>> {
        let value = self.buffer.get_byte(self.pos);

        let pos = self.pos;

        self.pos += 1;

        if self.buffer.beyond_buffer(self.pos) {
            self.load_next_chunk().await?;
        }

        Ok(Some(NextValue { value, pos }))
    }

    fn get_pos(&self) -> usize {
        self.pos
    }

    async fn get_slice_to_current_pos(&self, from_pos: usize) -> std::io::Result<Vec<u8>> {
        if self.buffer.offset <= from_pos {
            return Ok(self.buffer.get_slice(from_pos, self.pos).to_vec());
        }

        return self.load_slice_from_file(from_pos, self.pos).await;
    }

    async fn get_slice_to_end(&self, from_pos: usize) -> std::io::Result<Vec<u8>> {
        self.load_slice_from_file(from_pos, self.pos).await
    }

    async fn advance(&mut self, amount: usize) -> std::io::Result<Option<Vec<u8>>> {
        let end_pos = self.pos + amount;

        if end_pos >= self.file_size {
            return Ok(None);
        }

        if self.buffer.beyond_buffer(end_pos) {
            let result = self.load_slice_from_file(self.pos, end_pos).await?;

            self.pos = end_pos;
            return Ok(Some(result));
        }

        Ok(Some(self.buffer.get_slice(self.pos, end_pos).to_vec()))
    }
}
