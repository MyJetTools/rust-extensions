pub struct HexArray(String);

impl HexArray {
    pub fn from_slice(src: &[u8]) -> Self {
        src.into()
    }

    pub fn from_slice_uppercase(src: &[u8]) -> Self {
        let result = super::utils::array_of_bytes_to_hex_upper_case(src);
        HexArray(result)
    }

    pub fn new_unchecked(src: String) -> Self {
        HexArray(src)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }

    pub fn to_string(&self) -> String {
        self.0.clone()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        super::utils::hex_array_to_bytes(self.0.as_str())
    }
}

impl Into<HexArray> for &'_ [u8] {
    fn into(self) -> HexArray {
        let result = super::utils::array_of_bytes_to_hex(self);
        HexArray(result)
    }
}

impl Into<HexArray> for &'_ str {
    fn into(self) -> HexArray {
        for i in self.chars() {
            if !i.is_ascii_hexdigit() {
                panic!("Invalid hex digit {}", i);
            }
        }
        HexArray(self.to_string())
    }
}

impl Into<HexArray> for String {
    fn into(self) -> HexArray {
        for i in self.chars() {
            if !i.is_ascii_hexdigit() {
                panic!("Invalid hex digit {}", i);
            }
        }
        HexArray(self)
    }
}

impl std::fmt::Display for HexArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::HexArray;

    #[test]
    fn tests() {
        let src = vec![0x01, 0x02, 0x03, 0x04, 0x0a];
        let hex: HexArray = src.as_slice().into();

        assert_eq!(hex.as_str(), "010203040a");
        let dest = hex.to_bytes();
        assert_eq!(src, dest);
    }
}
