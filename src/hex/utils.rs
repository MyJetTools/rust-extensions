pub const HEX_CHARS: &[u8] = b"0123456789abcdef";
pub const HEX_CHARS_UPPER_CASE: &[u8] = b"0123456789ABCDEF";

pub fn hex_symbol_to_byte(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => panic!("Invalid hex character"),
    }
}

pub fn hex_to_byte(src: &[u8]) -> u8 {
    if src.len() == 1 {
        return hex_symbol_to_byte(src[0]);
    }

    return hex_symbol_to_byte(src[0]) * 16 + hex_symbol_to_byte(src[1]);
}

pub fn hex_to_word(src: &[u8]) -> u16 {
    match src.len() {
        1 => {
            return hex_symbol_to_byte(src[0]) as u16;
        }
        2 => {
            return hex_to_byte(src) as u16;
        }
        3 => return hex_symbol_to_byte(src[0]) as u16 * 256 + hex_to_byte(&src[1..]) as u16,
        4 => return hex_to_byte(&src[0..2]) as u16 * 256 + hex_to_byte(&src[2..]) as u16,
        _ => {
            panic!("Invalid hex word length {}", src.len());
        }
    }
}

pub fn word_to_hex(src: u16) -> String {
    let mut result = String::with_capacity(4);

    result.push(HEX_CHARS[(src >> 12) as usize] as char);

    result.push(HEX_CHARS[((src >> 8) & 0x0f) as usize] as char);
    result.push(HEX_CHARS[((src >> 4) & 0x0f) as usize] as char);
    result.push(HEX_CHARS[(src & 0x0f) as usize] as char);

    result
}

/*
pub fn word_to_hex_uppercase(src: u16) -> [u8; 4] {
    let mut result = String::with_capacity(4);

    result.push(HEX_CHARS_UPPER_CASE[(src >> 12) as usize] as char);

    result.push(HEX_CHARS_UPPER_CASE[((src >> 8) & 0x0f) as usize] as char);
    result.push(HEX_CHARS_UPPER_CASE[((src >> 4) & 0x0f) as usize] as char);
    result.push(HEX_CHARS_UPPER_CASE[(src & 0x0f) as usize] as char);

    result
}
 */
pub fn hex_array_to_bytes(hex_array: &str) -> Vec<u8> {
    let bytes = hex_array.as_bytes();

    if bytes.len() % 2 != 0 {
        panic!("Invalid hex array length {}", bytes.len());
    }

    let mut i = 0;

    let mut result = Vec::with_capacity(bytes.len() / 2);

    while i < bytes.len() {
        result.push(hex_to_byte(&bytes[i..i + 2]));
        i += 2;
    }

    result
}

pub fn byte_to_hex(b: u8) -> [u8; 2] {
    [HEX_CHARS[(b >> 4) as usize], HEX_CHARS[(b & 0x0f) as usize]]
}

pub fn byte_to_hex_upper_case(b: u8) -> [u8; 2] {
    [
        HEX_CHARS_UPPER_CASE[(b >> 4) as usize],
        HEX_CHARS_UPPER_CASE[(b & 0x0f) as usize],
    ]
}

pub fn array_of_bytes_to_hex(src: &[u8]) -> String {
    let mut result = Vec::with_capacity(src.len() * 2);

    for b in src {
        result.push(HEX_CHARS[(*b >> 4) as usize]);
        result.push(HEX_CHARS[(*b & 0x0f) as usize]);
    }

    unsafe { return String::from_utf8_unchecked(result) }
}

pub fn array_of_bytes_to_hex_upper_case(src: &[u8]) -> String {
    let mut result = Vec::with_capacity(src.len() * 2);

    for b in src {
        result.push(HEX_CHARS_UPPER_CASE[(*b >> 4) as usize]);
        result.push(HEX_CHARS_UPPER_CASE[(*b & 0x0f) as usize]);
    }

    unsafe { return String::from_utf8_unchecked(result) }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_hex_chars() {
        assert_eq!(super::hex_symbol_to_byte(b'0'), 0);
        assert_eq!(super::hex_symbol_to_byte(b'1'), 1);
        assert_eq!(super::hex_symbol_to_byte(b'2'), 2);
        assert_eq!(super::hex_symbol_to_byte(b'3'), 3);
        assert_eq!(super::hex_symbol_to_byte(b'4'), 4);
        assert_eq!(super::hex_symbol_to_byte(b'5'), 5);
        assert_eq!(super::hex_symbol_to_byte(b'6'), 6);
        assert_eq!(super::hex_symbol_to_byte(b'7'), 7);
        assert_eq!(super::hex_symbol_to_byte(b'8'), 8);
        assert_eq!(super::hex_symbol_to_byte(b'9'), 9);
        assert_eq!(super::hex_symbol_to_byte(b'a'), 10);
        assert_eq!(super::hex_symbol_to_byte(b'b'), 11);
        assert_eq!(super::hex_symbol_to_byte(b'c'), 12);
        assert_eq!(super::hex_symbol_to_byte(b'd'), 13);
        assert_eq!(super::hex_symbol_to_byte(b'e'), 14);
        assert_eq!(super::hex_symbol_to_byte(b'f'), 15);
        assert_eq!(super::hex_symbol_to_byte(b'A'), 10);
        assert_eq!(super::hex_symbol_to_byte(b'B'), 11);
        assert_eq!(super::hex_symbol_to_byte(b'C'), 12);
        assert_eq!(super::hex_symbol_to_byte(b'D'), 13);
        assert_eq!(super::hex_symbol_to_byte(b'E'), 14);
        assert_eq!(super::hex_symbol_to_byte(b'F'), 15);
    }

    #[test]
    fn test_hex_bytes() {
        assert_eq!(super::hex_to_byte(b"0"), 0);
        assert_eq!(super::hex_to_byte(b"1"), 1);
        assert_eq!(super::hex_to_byte(b"2"), 2);
        assert_eq!(super::hex_to_byte(b"3"), 3);
        assert_eq!(super::hex_to_byte(b"4"), 4);
        assert_eq!(super::hex_to_byte(b"5"), 5);
        assert_eq!(super::hex_to_byte(b"6"), 6);
        assert_eq!(super::hex_to_byte(b"7"), 7);
        assert_eq!(super::hex_to_byte(b"8"), 8);
        assert_eq!(super::hex_to_byte(b"9"), 9);

        assert_eq!(super::hex_to_byte(b"00"), 0);
        assert_eq!(super::hex_to_byte(b"01"), 1);
        assert_eq!(super::hex_to_byte(b"02"), 2);
        assert_eq!(super::hex_to_byte(b"03"), 3);
        assert_eq!(super::hex_to_byte(b"04"), 4);
        assert_eq!(super::hex_to_byte(b"05"), 5);

        assert_eq!(super::hex_to_byte(b"0a"), 10);
        assert_eq!(super::hex_to_byte(b"0A"), 10);

        assert_eq!(super::hex_to_byte(b"0f"), 15);
        assert_eq!(super::hex_to_byte(b"0F"), 15);

        assert_eq!(super::hex_to_byte(b"10"), 16);
        assert_eq!(super::hex_to_byte(b"10"), 16);

        assert_eq!(super::hex_to_byte(b"a1"), 161);
        assert_eq!(super::hex_to_byte(b"A1"), 161);

        assert_eq!(super::hex_to_byte(b"ff"), 255);
        assert_eq!(super::hex_to_byte(b"fF"), 255);
    }

    #[test]
    pub fn test_hex_words() {
        assert_eq!(super::hex_to_word(b"0"), 0);
        assert_eq!(super::hex_to_word(b"00"), 0);
        assert_eq!(super::hex_to_word(b"f3db"), 62427);
        assert_eq!(super::hex_to_word(b"f3Db"), 62427);
    }

    #[test]
    pub fn test_byte_to_hex() {
        assert_eq!(super::byte_to_hex(0), [b'0', b'0']);
        assert_eq!(super::byte_to_hex(1), [b'0', b'1']);
        assert_eq!(super::byte_to_hex(2), [b'0', b'2']);
        assert_eq!(super::byte_to_hex(3), [b'0', b'3']);
        assert_eq!(super::byte_to_hex(4), [b'0', b'4']);
        assert_eq!(super::byte_to_hex(5), [b'0', b'5']);

        assert_eq!(super::byte_to_hex(10), [b'0', b'a']);
        assert_eq!(super::byte_to_hex(15), [b'0', b'f']);
        assert_eq!(super::byte_to_hex(16), [b'1', b'0']);
        assert_eq!(super::byte_to_hex(161), [b'a', b'1']);
        assert_eq!(super::byte_to_hex(255), [b'f', b'f']);
    }

    #[test]
    pub fn test_hex_array() {
        let hex_array = "00010203ed46";
        let as_vec = super::hex_array_to_bytes(hex_array);
        assert_eq!(as_vec, vec![0, 1, 2, 3, 237, 70]);

        let result_hex = super::array_of_bytes_to_hex(&as_vec);

        assert_eq!(hex_array, result_hex);

        let result_hex = super::array_of_bytes_to_hex_upper_case(&as_vec);

        assert_eq!(hex_array.to_ascii_uppercase(), result_hex);
    }
}
