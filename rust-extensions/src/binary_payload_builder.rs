use crate::SliceOrVec;

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
                data.extend_from_slice(value.to_be_bytes().as_slice());
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
                data.extend_from_slice(value.to_be_bytes().as_slice());
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
                data.extend_from_slice(value.to_be_bytes().as_slice());
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
                data.extend_from_slice(value.to_be_bytes().as_slice());
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
                data.extend_from_slice(value.to_be_bytes().as_slice());
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
                data.extend_from_slice(value.to_be_bytes().as_slice());
            }
        }
    }
}

impl<'s> Into<SliceOrVec<'s, u8>> for BinaryPayloadBuilder<'s> {
    fn into(self) -> SliceOrVec<'s, u8> {
        match self {
            BinaryPayloadBuilder::AsSlice(data, _) => SliceOrVec::AsSlice(data),
            BinaryPayloadBuilder::AsVec(data) => SliceOrVec::AsVec(data),
        }
    }
}
