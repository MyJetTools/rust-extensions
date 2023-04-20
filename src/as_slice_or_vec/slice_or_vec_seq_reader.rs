use std::io::Read;

use crate::AsSliceOrVec;

pub struct SliceOrVecSeqReader<'s, T: Clone> {
    src: AsSliceOrVec<'s, T>,
    offset: usize,
}

impl<'s, T: Clone> SliceOrVecSeqReader<'s, T> {
    pub fn new(src: AsSliceOrVec<'s, T>) -> Self {
        Self { src, offset: 0 }
    }

    pub fn get_offset(&self) -> usize {
        self.offset
    }

    fn shift_offset_forward(&mut self, len: usize) {
        self.offset += len;
    }
}

impl<'s> Read for SliceOrVecSeqReader<'s, u8> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = self.src.get_len();
        let offset = self.get_offset();

        if offset >= len {
            return Ok(0);
        }

        let remain = len - offset;

        let read_len = if remain < buf.len() {
            buf[..remain].copy_from_slice(&self.src.as_slice()[offset..]);
            remain
        } else {
            buf.copy_from_slice(&self.src.as_slice()[offset..offset + buf.len()]);
            buf.len()
        };

        self.shift_offset_forward(read_len);

        Ok(read_len)
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
