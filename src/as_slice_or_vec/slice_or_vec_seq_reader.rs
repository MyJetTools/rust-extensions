use std::io::{Read, Seek};

use crate::AsSliceOrVec;

pub struct SliceOrVecSeqReader<'s, T: Clone> {
    inner: AsSliceOrVec<'s, T>,
    offset: usize,
}

impl<'s, T: Clone> SliceOrVecSeqReader<'s, T> {
    pub fn new(src: AsSliceOrVec<'s, T>) -> Self {
        Self {
            inner: src,
            offset: 0,
        }
    }

    pub fn get_offset(&self) -> usize {
        self.offset
    }
}

impl<'s> Read for SliceOrVecSeqReader<'s, u8> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = self.inner.get_len();
        let offset = self.get_offset();

        if offset >= len {
            return Ok(0);
        }

        let remain = len - offset;

        let read_len = if remain < buf.len() {
            buf[..remain].copy_from_slice(&self.inner.as_slice()[offset..offset + remain]);
            remain
        } else {
            buf.copy_from_slice(&self.inner.as_slice()[offset..offset + buf.len()]);
            buf.len()
        };

        self.offset += read_len;

        Ok(read_len)
    }
}

impl<'s> Seek for SliceOrVecSeqReader<'s, u8> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            std::io::SeekFrom::Start(value) => {
                self.offset = value as usize;
                return Ok(value);
            }
            std::io::SeekFrom::End(value) => self.inner.get_len() as i64 + value,
            std::io::SeekFrom::Current(value) => self.offset as i64 + value,
        };

        if new_pos < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid seek to a negative position",
            ));
        }

        let new_pos = new_pos as usize;

        if new_pos > self.inner.get_len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid seek to a position that is larger than the length of the slice",
            ));
        }

        self.offset = new_pos;

        Ok(self.offset as u64)
    }
}

impl<'s, T: Clone> Into<SliceOrVecSeqReader<'s, T>> for AsSliceOrVec<'s, T> {
    fn into(self) -> SliceOrVecSeqReader<'s, T> {
        SliceOrVecSeqReader::new(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::{AsSliceOrVec, SliceOrVecSeqReader};
    use std::io::Read;

    #[test]
    fn test_read_sequences() {
        let buffer = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let mut buffer: SliceOrVecSeqReader<'_, u8> = AsSliceOrVec::create_as_vec(buffer).into();

        let mut out_buffer = [0u8; 2];

        let result = buffer.read(&mut out_buffer).unwrap();

        assert_eq!(result, 2);
        assert_eq!(out_buffer, [1, 2]);

        let result = buffer.read(&mut out_buffer).unwrap();

        assert_eq!(result, 2);
        assert_eq!(out_buffer, [3, 4]);

        let mut out_buffer = [0u8; 7];

        let result = buffer.read(&mut out_buffer).unwrap();

        assert_eq!(result, 6);
        assert_eq!(out_buffer[0..6], [5, 6, 7, 8, 9, 10]);

        let result = buffer.read(&mut out_buffer).unwrap();

        assert_eq!(result, 0);
    }
}
