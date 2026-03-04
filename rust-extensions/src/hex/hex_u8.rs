#[derive(Debug, Clone, Copy)]
pub struct HexU8([u8; 2]);

impl HexU8 {
    pub fn new(decimal_value: u8) -> Self {
        Self(super::utils::byte_to_hex(decimal_value))
    }

    pub fn new_uppercase(decimal_value: u8) -> Self {
        Self(super::utils::byte_to_hex_upper_case(decimal_value))
    }

    pub fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }

    pub fn to_u8(&self) -> u8 {
        super::utils::hex_to_byte(&self.0)
    }
}

impl Into<HexU8> for u8 {
    fn into(self) -> HexU8 {
        HexU8::new(self)
    }
}

impl<'s> TryFrom<&'s str> for HexU8 {
    type Error = String;
    fn try_from(value: &'s str) -> Result<Self, Self::Error> {
        let value_bytes = value.as_bytes();
        match value_bytes.len() {
            1 => {
                if !value_bytes[0].is_ascii_hexdigit() {
                    return Err(format!("Invalid hex digit: {}", value));
                }
                let mut result = [b'0', value_bytes[0]];
                result[1] = value_bytes[0];
                return Ok(HexU8(result));
            }
            2 => {
                if !value_bytes[0].is_ascii_hexdigit() {
                    return Err(format!("Invalid hex digit: {}", value));
                }

                if !value_bytes[1].is_ascii_hexdigit() {
                    return Err(format!("Invalid hex digit: {}", value));
                }
                let result = [value_bytes[0], value_bytes[1]];
                return Ok(HexU8(result));
            }
            _ => {
                return Err(format!("Invalid hex u8 string len={}", value_bytes.len()));
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn tests() {
        use super::HexU8;

        let a = HexU8::new(0x12);
        assert_eq!(a.as_str(), "12");
        assert_eq!(a.to_u8(), 0x12);

        let a: HexU8 = "12".try_into().unwrap();
        assert_eq!(a.as_str(), "12");
        assert_eq!(a.to_u8(), 0x12);

        let a: HexU8 = "2".try_into().unwrap();
        assert_eq!(a.as_str(), "02");
        assert_eq!(a.to_u8(), 0x02);

        let hex_u8 = HexU8::new(0xfa);
        assert_eq!(hex_u8.as_str(), "fa");

        let hex_u8 = HexU8::new_uppercase(0xbc);
        assert_eq!(hex_u8.as_str(), "BC");
    }
}
