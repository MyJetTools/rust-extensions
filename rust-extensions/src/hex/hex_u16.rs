#[derive(Debug, Clone, Copy)]
pub struct HexU16([u8; 4]);

impl HexU16 {
    pub fn new(decimal_value: u16) -> Self {
        let value_as_bytes = decimal_value.to_be_bytes();
        let mut result = [0u8; 4];
        result[..2].copy_from_slice(super::utils::byte_to_hex(value_as_bytes[0]).as_slice());
        result[2..].copy_from_slice(super::utils::byte_to_hex(value_as_bytes[1]).as_slice());
        Self(result)
    }

    pub fn new_uppercase(decimal_value: u16) -> Self {
        let value_as_bytes = decimal_value.to_be_bytes();
        let mut result = [0u8; 4];
        result[..2]
            .copy_from_slice(super::utils::byte_to_hex_upper_case(value_as_bytes[0]).as_slice());
        result[2..]
            .copy_from_slice(super::utils::byte_to_hex_upper_case(value_as_bytes[1]).as_slice());
        Self(result)
    }

    pub fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }

    pub fn to_u16(&self) -> u16 {
        u16::from_str_radix(self.as_str(), 16).unwrap()
    }
}

impl Into<HexU16> for u16 {
    fn into(self) -> HexU16 {
        HexU16::new(self)
    }
}

impl<'s> TryFrom<&'s str> for HexU16 {
    type Error = String;
    fn try_from(value: &'s str) -> Result<Self, Self::Error> {
        let value_bytes = value.as_bytes();
        match value_bytes.len() {
            1 => {
                if !value_bytes[0].is_ascii_hexdigit() {
                    return Err(format!("Invalid hex digit: {}", value));
                }
                let mut result = [b'0', b'0', b'0', value_bytes[0]];
                result[1] = value_bytes[0];
                return Ok(HexU16(result));
            }
            2 => {
                if !value_bytes[0].is_ascii_hexdigit() {
                    return Err(format!("Invalid hex digit: {}", value));
                }

                if !value_bytes[1].is_ascii_hexdigit() {
                    return Err(format!("Invalid hex digit: {}", value));
                }
                let result = [b'0', b'0', value_bytes[0], value_bytes[1]];
                return Ok(HexU16(result));
            }
            3 => {
                if !value_bytes[0].is_ascii_hexdigit() {
                    return Err(format!("Invalid hex digit: {}", value));
                }

                if !value_bytes[1].is_ascii_hexdigit() {
                    return Err(format!("Invalid hex digit: {}", value));
                }
                if !value_bytes[2].is_ascii_hexdigit() {
                    return Err(format!("Invalid hex digit: {}", value));
                }
                let result = [b'0', value_bytes[0], value_bytes[1], value_bytes[2]];
                return Ok(HexU16(result));
            }
            4 => {
                if !value_bytes[0].is_ascii_hexdigit() {
                    return Err(format!("Invalid hex digit: {}", value));
                }

                if !value_bytes[1].is_ascii_hexdigit() {
                    return Err(format!("Invalid hex digit: {}", value));
                }
                if !value_bytes[2].is_ascii_hexdigit() {
                    return Err(format!("Invalid hex digit: {}", value));
                }
                if !value_bytes[3].is_ascii_hexdigit() {
                    return Err(format!("Invalid hex digit: {}", value));
                }
                let result = [
                    value_bytes[0],
                    value_bytes[1],
                    value_bytes[2],
                    value_bytes[3],
                ];
                return Ok(HexU16(result));
            }
            _ => {
                return Err(format!("Invalid hex u16 string len={}", value_bytes.len()));
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn tests() {
        use super::HexU16;

        let a = HexU16::new(0x1234);
        assert_eq!(a.as_str(), "1234");

        assert_eq!(a.to_u16(), 0x1234);

        let hex_u16 = HexU16::new(0xfabc);
        assert_eq!(hex_u16.as_str(), "fabc");

        let hex_u16 = HexU16::new_uppercase(0xfabc);
        assert_eq!(hex_u16.as_str(), "FABC");

        let hex_u16: HexU16 = "fabc".try_into().unwrap();
        assert_eq!(hex_u16.as_str(), "fabc");
        assert_eq!(hex_u16.to_u16(), 0xfabc);
    }
}
