use crate::SliceOrVec;

/// Appends integers to a payload - either to a growing `Vec<u8>` or into a
/// buffer given by the caller.
///
/// Multi-byte integers are written as little-endian, and both modes give the
/// very same bytes for the same writes.
pub enum BinaryPayloadBuilder<'s> {
    AsSlice(&'s mut [u8], usize),
    AsVec(Vec<u8>),
}

impl<'s> BinaryPayloadBuilder<'s> {
    pub fn new_as_slice(data: &'s mut [u8]) -> Self {
        Self::AsSlice(data, 0)
    }

    pub fn new_as_vec() -> Self {
        Self::AsVec(Vec::new())
    }

    pub fn write_u8(&mut self, value: u8) {
        match self {
            BinaryPayloadBuilder::AsSlice(data, offset) => {
                let value_to_write = data.get_mut(*offset).unwrap();
                *value_to_write = value;
                *offset += 1;
            }
            BinaryPayloadBuilder::AsVec(data) => {
                data.push(value);
            }
        }
    }

    pub fn write_i8(&mut self, value: i8) {
        match self {
            BinaryPayloadBuilder::AsSlice(data, offset) => {
                let value_to_write = data.get_mut(*offset).unwrap();
                *value_to_write = value as u8;
                *offset += 1;
            }
            BinaryPayloadBuilder::AsVec(data) => {
                data.push(value as u8);
            }
        }
    }

    pub fn write_u16(&mut self, value: u16) {
        match self {
            BinaryPayloadBuilder::AsSlice(data, offset) => {
                const SIZE: usize = 2;

                let value_to_write = &mut data[*offset..*offset + SIZE];
                value_to_write.copy_from_slice(value.to_le_bytes().as_slice());
                *offset += SIZE;
            }
            BinaryPayloadBuilder::AsVec(data) => {
                data.extend_from_slice(value.to_le_bytes().as_slice());
            }
        }
    }

    pub fn write_i16(&mut self, value: i16) {
        match self {
            BinaryPayloadBuilder::AsSlice(data, offset) => {
                const SIZE: usize = 2;

                let value_to_write = &mut data[*offset..*offset + SIZE];
                value_to_write.copy_from_slice(value.to_le_bytes().as_slice());
                *offset += SIZE;
            }
            BinaryPayloadBuilder::AsVec(data) => {
                data.extend_from_slice(value.to_le_bytes().as_slice());
            }
        }
    }

    pub fn write_u32(&mut self, value: u32) {
        match self {
            BinaryPayloadBuilder::AsSlice(data, offset) => {
                const SIZE: usize = 4;

                let value_to_write = &mut data[*offset..*offset + SIZE];
                value_to_write.copy_from_slice(value.to_le_bytes().as_slice());
                *offset += SIZE;
            }
            BinaryPayloadBuilder::AsVec(data) => {
                data.extend_from_slice(value.to_le_bytes().as_slice());
            }
        }
    }

    pub fn write_i32(&mut self, value: i32) {
        match self {
            BinaryPayloadBuilder::AsSlice(data, offset) => {
                const SIZE: usize = 4;

                let value_to_write = &mut data[*offset..*offset + SIZE];
                value_to_write.copy_from_slice(value.to_le_bytes().as_slice());
                *offset += SIZE;
            }
            BinaryPayloadBuilder::AsVec(data) => {
                data.extend_from_slice(value.to_le_bytes().as_slice());
            }
        }
    }

    pub fn write_u64(&mut self, value: u64) {
        match self {
            BinaryPayloadBuilder::AsSlice(data, offset) => {
                const SIZE: usize = 8;

                let value_to_write = &mut data[*offset..*offset + SIZE];
                value_to_write.copy_from_slice(value.to_le_bytes().as_slice());
                *offset += SIZE;
            }
            BinaryPayloadBuilder::AsVec(data) => {
                data.extend_from_slice(value.to_le_bytes().as_slice());
            }
        }
    }

    pub fn write_i64(&mut self, value: i64) {
        match self {
            BinaryPayloadBuilder::AsSlice(data, offset) => {
                const SIZE: usize = 8;

                let value_to_write = &mut data[*offset..*offset + SIZE];
                value_to_write.copy_from_slice(value.to_le_bytes().as_slice());
                *offset += SIZE;
            }
            BinaryPayloadBuilder::AsVec(data) => {
                data.extend_from_slice(value.to_le_bytes().as_slice());
            }
        }
    }
}

impl<'s> Into<SliceOrVec<'s, u8>> for BinaryPayloadBuilder<'s> {
    fn into(self) -> SliceOrVec<'s, u8> {
        match self {
            // Only what was written - exactly as the `Vec` gives it. The rest of
            // the buffer is not a part of the payload.
            BinaryPayloadBuilder::AsSlice(data, offset) => SliceOrVec::AsSlice(&data[..offset]),
            BinaryPayloadBuilder::AsVec(data) => SliceOrVec::AsVec(data),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::SliceOrVec;

    use super::BinaryPayloadBuilder;

    /// Does the same write in both modes and expects the same bytes from each.
    fn assert_written(write: impl Fn(&mut BinaryPayloadBuilder), expected: &[u8]) {
        let mut builder = BinaryPayloadBuilder::new_as_vec();
        write(&mut builder);
        let as_vec: SliceOrVec<u8> = builder.into();
        assert!(as_vec.is_vec());
        assert_eq!(as_vec.as_slice(), expected);

        let mut buffer = vec![0u8; expected.len()];
        let mut builder = BinaryPayloadBuilder::new_as_slice(&mut buffer);
        write(&mut builder);
        let as_slice: SliceOrVec<u8> = builder.into();
        assert!(as_slice.is_slice());
        assert_eq!(as_slice.as_slice(), expected);
    }

    #[test]
    fn integers_are_written_as_little_endian_in_both_modes() {
        assert_written(|builder| builder.write_u8(0x01), &[0x01]);
        assert_written(|builder| builder.write_i8(-2), &[0xfe]);

        assert_written(|builder| builder.write_u16(0x0102), &[0x02, 0x01]);
        assert_written(|builder| builder.write_i16(-2), &[0xfe, 0xff]);

        assert_written(
            |builder| builder.write_u32(0x0102_0304),
            &[0x04, 0x03, 0x02, 0x01],
        );
        assert_written(|builder| builder.write_i32(-2), &[0xfe, 0xff, 0xff, 0xff]);

        assert_written(
            |builder| builder.write_u64(0x0102_0304_0506_0708),
            &[0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01],
        );
        assert_written(
            |builder| builder.write_i64(-2),
            &[0xfe, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        );
    }

    #[test]
    fn writes_follow_each_other() {
        assert_written(
            |builder| {
                builder.write_u16(0x0102);
                builder.write_u32(0x0304_0506);
                builder.write_u8(0x07);
            },
            &[2, 1, 6, 5, 4, 3, 7],
        );
    }

    #[test]
    fn what_is_written_is_read_back_by_from_le_bytes() {
        let mut builder = BinaryPayloadBuilder::new_as_vec();
        builder.write_u64(123_456);
        builder.write_u32(789);
        let payload: SliceOrVec<u8> = builder.into();
        let payload = payload.as_slice();

        assert_eq!(
            u64::from_le_bytes(payload[0..8].try_into().unwrap()),
            123_456
        );
        assert_eq!(u32::from_le_bytes(payload[8..12].try_into().unwrap()), 789);
    }

    #[test]
    fn slice_mode_gives_back_what_was_written_and_not_the_whole_buffer() {
        let mut buffer = [0xffu8; 8];

        let mut builder = BinaryPayloadBuilder::new_as_slice(&mut buffer);
        builder.write_u16(0x0102);
        builder.write_u32(0x0304_0506);
        let payload: SliceOrVec<u8> = builder.into();

        assert!(payload.is_slice());
        assert_eq!(payload.as_slice(), &[2, 1, 6, 5, 4, 3]);

        // The tail nothing was written into is left as it was.
        assert_eq!(buffer, [2, 1, 6, 5, 4, 3, 0xff, 0xff]);
    }

    #[test]
    fn nothing_written_is_an_empty_payload_in_both_modes() {
        let payload: SliceOrVec<u8> = BinaryPayloadBuilder::new_as_vec().into();
        assert_eq!(payload.get_len(), 0);

        let mut buffer = [0xffu8; 8];
        let payload: SliceOrVec<u8> = BinaryPayloadBuilder::new_as_slice(&mut buffer).into();
        assert_eq!(payload.get_len(), 0);
    }
}
