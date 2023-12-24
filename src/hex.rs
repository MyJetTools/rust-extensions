pub trait ToHex {
    fn to_hex(&self) -> String;
}

pub trait FromHex {
    fn from_hex(&self) -> u32;
}
impl ToHex for u32 {
    fn to_hex(&self) -> String {
        let value = *self;
        let bytes = value.to_be_bytes();

        if value < 256 {
            return hex::encode_upper(&bytes[3..4]);
        }
        if value < 256 * 256 {
            return hex::encode_upper(&bytes[2..4]);
        }

        if value < 256 * 256 * 256 {
            return hex::encode_upper(&bytes[1..4]);
        }

        hex::encode_upper(bytes)
    }
}

impl<'s> FromHex for &'s str {
    fn from_hex(&self) -> u32 {
        let mut result = [0u8; 4];
        if self.len() == 2 {
            hex::decode_to_slice(self, &mut result[3..4]).unwrap();
        } else if self.len() == 4 {
            hex::decode_to_slice(self, &mut result[2..4]).unwrap();
        } else if self.len() == 6 {
            hex::decode_to_slice(self, &mut result[1..4]).unwrap();
        } else {
            hex::decode_to_slice(self, &mut result).unwrap();
        }

        u32::from_be_bytes(result)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test() {
        for src in 0..65535 * 20 {
            let result = src.to_hex();
            let dest = result.as_str().from_hex();
            assert_eq!(src, dest);
        }
    }

    #[test]
    fn test_word() {
        let src: u32 = 65535 * 65535;
        let result = src.to_hex();
        assert_eq!("FFFE0001", result.as_str());

        let dest = result.as_str().from_hex();

        assert_eq!(src, dest);
    }

    #[test]
    fn test_3_digits() {
        let src = "00FE0001".from_hex();
        let result = src.to_hex();
        assert_eq!("FE0001", result.as_str());

        let dest = result.as_str().from_hex();

        assert_eq!(src, dest);
    }
}
