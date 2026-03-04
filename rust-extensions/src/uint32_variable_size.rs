#[derive(Debug, Clone, Copy)]
pub struct UInt32VariableSize(u32);

pub enum ParseUInt32VariableSizeResult {
    Ok {
        value: UInt32VariableSize,
        size: usize,
    },
    NotEnoughDataInBuffer(usize),
}

impl ParseUInt32VariableSizeResult {
    pub fn unwrap(&self) -> UInt32VariableSize {
        match self {
            ParseUInt32VariableSizeResult::Ok { value, .. } => *value,
            ParseUInt32VariableSizeResult::NotEnoughDataInBuffer(size) => {
                panic!(
                    "Not enough data size to get UInt32VariableSizeValue. Required size is {}",
                    size
                );
            }
        }
    }
}

impl UInt32VariableSize {
    const MAX_1_BYTE_VALUE: u32 = 64;
    const MAX_2_BYTES_VALUE: u32 = 16384;
    const MAX_3_BYTES_VALUE: u32 = 4194303;
    const MAX_4_BYTES_VALUE: u32 = 1073741823;

    const BYTES_2_MASK: u8 = 64;
    const BYTES_3_MASK: u8 = 128;
    const BYTES_4_MASK: u8 = 192;

    const UNMASK_VALUE: u8 = 63;

    pub fn new(value: u32) -> Self {
        if value >= Self::MAX_4_BYTES_VALUE {
            panic!(
                "Max value can be {}. Provided value is {}",
                Self::MAX_4_BYTES_VALUE - 1,
                value,
            );
        }
        Self(value)
    }

    pub fn get_value(&self) -> u32 {
        self.0
    }

    pub fn serialize(&self, out: &mut Vec<u8>) {
        if self.0 < Self::MAX_1_BYTE_VALUE {
            out.push(self.0 as u8);
            return;
        }

        if self.0 < Self::MAX_2_BYTES_VALUE {
            let bytes = self.0.to_be_bytes();
            out.push(Self::BYTES_2_MASK | bytes[2]);
            out.push(bytes[3]);

            return;
        }

        if self.0 < Self::MAX_3_BYTES_VALUE {
            let bytes = self.0.to_be_bytes();
            out.push(bytes[1] | Self::BYTES_3_MASK);
            out.push(bytes[2]);
            out.push(bytes[3]);

            return;
        }

        if self.0 < Self::MAX_4_BYTES_VALUE {
            let mut bytes = self.0.to_be_bytes();

            bytes[0] = bytes[0] | Self::BYTES_4_MASK;
            out.extend_from_slice(&bytes);
            return;
        }

        panic!(
            "Max value can be {}. Provided value is {}",
            Self::MAX_4_BYTES_VALUE,
            self.0
        );
    }

    pub fn from_slice(slice: &[u8]) -> ParseUInt32VariableSizeResult {
        if slice.len() == 0 {
            panic!("Slice is empty");
        }
        let first_byte = slice[0];

        let first_byte_as_u32 = first_byte as u32;

        if first_byte_as_u32 < Self::MAX_1_BYTE_VALUE {
            return ParseUInt32VariableSizeResult::Ok {
                value: Self::new(first_byte as u32),
                size: 1,
            };
        }

        let size_mask_byte = first_byte & 192;

        match size_mask_byte {
            Self::BYTES_2_MASK => {
                const DATA_SIZE: usize = 2;
                if slice.len() < DATA_SIZE {
                    return ParseUInt32VariableSizeResult::NotEnoughDataInBuffer(DATA_SIZE);
                }

                let mut bytes = [0u8; 4];

                bytes[2] = first_byte & Self::UNMASK_VALUE;
                bytes[3] = slice[1];

                let value = u32::from_be_bytes(bytes);

                return ParseUInt32VariableSizeResult::Ok {
                    value: Self(value),
                    size: DATA_SIZE,
                };
            }

            Self::BYTES_3_MASK => {
                const DATA_SIZE: usize = 3;
                if slice.len() < DATA_SIZE {
                    return ParseUInt32VariableSizeResult::NotEnoughDataInBuffer(DATA_SIZE);
                }

                let mut bytes = [0u8; 4];

                bytes[1] = first_byte & Self::UNMASK_VALUE;
                bytes[2] = slice[1];
                bytes[3] = slice[2];

                let value = u32::from_be_bytes(bytes);

                return ParseUInt32VariableSizeResult::Ok {
                    value: Self(value),
                    size: DATA_SIZE,
                };
            }

            Self::BYTES_4_MASK => {
                const DATA_SIZE: usize = 4;
                if slice.len() < DATA_SIZE {
                    return ParseUInt32VariableSizeResult::NotEnoughDataInBuffer(DATA_SIZE);
                }

                let mut bytes = [0u8; 4];

                bytes[0] = first_byte & Self::UNMASK_VALUE;
                bytes[1] = slice[1];
                bytes[2] = slice[2];
                bytes[3] = slice[3];

                let value = u32::from_be_bytes(bytes);

                return ParseUInt32VariableSizeResult::Ok {
                    value: Self(value),
                    size: DATA_SIZE,
                };
            }

            _ => {
                panic!("Should not be here")
            }
        }
    }
}

#[cfg(test)]
mod test {

    use super::UInt32VariableSize;

    #[test]
    fn test_one_byte_serialization() {
        for v in 0..64 {
            test_value(v, 1);
        }
    }

    #[test]
    fn test_two_bytes_in_buffer() {
        for v in 64..16384 {
            test_value(v, 2);
        }
    }

    #[test]
    fn test_three_bytes_in_buffer() {
        for v in 16384..4194303 {
            test_value(v, 3);
        }
    }

    #[test]
    fn test_4_bytes_in_buffer() {
        let mut v = 4194303;

        while v < 1073741823 {
            test_value(v, 4);
            v += 100;
        }

        test_value(1073741822, 4);
    }

    fn test_value(src_value: u32, data_size: usize) {
        let value = UInt32VariableSize::new(src_value);

        let mut bytes = Vec::new();

        value.serialize(&mut bytes);

        assert_eq!(bytes.len(), data_size);

        match UInt32VariableSize::from_slice(bytes.as_slice()) {
            crate::ParseUInt32VariableSizeResult::Ok { value, size } => {
                assert_eq!(value.get_value(), src_value);
                assert_eq!(size, data_size);
            }
            crate::ParseUInt32VariableSizeResult::NotEnoughDataInBuffer(_) => {
                panic!("Should not be here")
            }
        }
    }
}
