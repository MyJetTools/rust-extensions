#[derive(Debug, Clone, Copy)]
pub struct HexU32([u8; 8]);

impl HexU32 {
    pub fn new(decimal_value: u32) -> Self {
        let value_as_bytes = decimal_value.to_be_bytes();
        let mut result = [0u8; 8];
        result[..2].copy_from_slice(super::utils::byte_to_hex(value_as_bytes[0]).as_slice());
        result[2..4].copy_from_slice(super::utils::byte_to_hex(value_as_bytes[1]).as_slice());
        result[4..6].copy_from_slice(super::utils::byte_to_hex(value_as_bytes[2]).as_slice());
        result[6..].copy_from_slice(super::utils::byte_to_hex(value_as_bytes[3]).as_slice());
        Self(result)
    }

    pub fn new_uppercase(decimal_value: u32) -> Self {
        let value_as_bytes = decimal_value.to_be_bytes();
        let mut result = [0u8; 8];
        result[..2]
            .copy_from_slice(super::utils::byte_to_hex_upper_case(value_as_bytes[0]).as_slice());
        result[2..4]
            .copy_from_slice(super::utils::byte_to_hex_upper_case(value_as_bytes[1]).as_slice());
        result[4..6]
            .copy_from_slice(super::utils::byte_to_hex_upper_case(value_as_bytes[2]).as_slice());
        result[6..]
            .copy_from_slice(super::utils::byte_to_hex_upper_case(value_as_bytes[3]).as_slice());
        Self(result)
    }

    pub fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }

    pub fn to_u32(&self) -> u32 {
        u32::from_str_radix(self.as_str(), 16).unwrap()
    }
}

impl Into<HexU32> for u32 {
    fn into(self) -> HexU32 {
        HexU32::new(self)
    }
}

impl<'s> TryFrom<&'s str> for HexU32 {
    type Error = String;
    fn try_from(value: &'s str) -> Result<Self, Self::Error> {
        let value_bytes = value.as_bytes();

        if value_bytes.len() > 8 {
            return Err(format!("Invalid hex u32 string len={}", value_bytes.len()));
        }

        for i in value_bytes {
            if !i.is_ascii_hexdigit() {
                return Err(format!("Invalid hex digit: {}", value));
            }
        }

        match value_bytes.len() {
            1 => {
                let mut result = [b'0', b'0', b'0', b'0', b'0', b'0', b'0', value_bytes[0]];
                result[1] = value_bytes[0];
                return Ok(HexU32(result));
            }
            2 => {
                let result = [
                    b'0',
                    b'0',
                    b'0',
                    b'0',
                    b'0',
                    b'0',
                    value_bytes[0],
                    value_bytes[1],
                ];
                return Ok(HexU32(result));
            }
            3 => {
                let result = [
                    b'0',
                    b'0',
                    b'0',
                    b'0',
                    b'0',
                    value_bytes[0],
                    value_bytes[1],
                    value_bytes[2],
                ];
                return Ok(HexU32(result));
            }
            4 => {
                let result = [
                    b'0',
                    b'0',
                    b'0',
                    b'0',
                    value_bytes[0],
                    value_bytes[1],
                    value_bytes[2],
                    value_bytes[3],
                ];
                return Ok(HexU32(result));
            }
            5 => {
                let result = [
                    b'0',
                    b'0',
                    b'0',
                    value_bytes[0],
                    value_bytes[1],
                    value_bytes[2],
                    value_bytes[3],
                    value_bytes[4],
                ];
                return Ok(HexU32(result));
            }
            6 => {
                let result = [
                    b'0',
                    b'0',
                    value_bytes[0],
                    value_bytes[1],
                    value_bytes[2],
                    value_bytes[3],
                    value_bytes[4],
                    value_bytes[5],
                ];
                return Ok(HexU32(result));
            }
            7 => {
                let result = [
                    b'0',
                    value_bytes[0],
                    value_bytes[1],
                    value_bytes[2],
                    value_bytes[3],
                    value_bytes[4],
                    value_bytes[5],
                    value_bytes[6],
                ];
                return Ok(HexU32(result));
            }

            8 => {
                let result = [
                    value_bytes[0],
                    value_bytes[1],
                    value_bytes[2],
                    value_bytes[3],
                    value_bytes[4],
                    value_bytes[5],
                    value_bytes[6],
                    value_bytes[7],
                ];
                return Ok(HexU32(result));
            }
            _ => {
                return Err(format!("Invalid hex u32 string len={}", value_bytes.len()));
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn tests() {
        use super::HexU32;

        let a = HexU32::new(0x1234);
        assert_eq!(a.as_str(), "00001234");

        assert_eq!(a.to_u32(), 0x1234);

        let hex_u16 = HexU32::new(0xfabc);
        assert_eq!(hex_u16.as_str(), "0000fabc");

        let hex_u16 = HexU32::new_uppercase(0xfabc);
        assert_eq!(hex_u16.as_str(), "0000FABC");

        let hex_u16: HexU32 = "fabc".try_into().unwrap();
        assert_eq!(hex_u16.as_str(), "0000fabc");
        assert_eq!(hex_u16.to_u32(), 0xfabc);
    }
}
