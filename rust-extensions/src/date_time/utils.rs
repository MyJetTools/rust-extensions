const START_ZERO: u8 = '0' as u8;
const START_NINE: u8 = '9' as u8;

pub fn parse_two_digits(src: &[u8]) -> Option<u32> {
    let result = parse_number(*src.get(0)?)? * 10 + parse_number(*src.get(1)?)?;
    Some(result as u32)
}

#[inline]
pub fn parse_number(src: u8) -> Option<u32> {
    if src < START_ZERO || src > START_NINE {
        return None;
    }

    return Some((src - START_ZERO) as u32);
}

#[inline]
pub fn parse_four_digits(src: &[u8]) -> Option<i32> {
    let result = parse_number(src[0])? * 1000
        + parse_number(src[1])? * 100
        + parse_number(src[2])? * 10
        + parse_number(src[3])?;

    Some(result as i32)
}

/// Writes `value` with leading zeros, `digits` wide. It goes right into `dest`, without the
/// allocation of a `to_string()`: the serde format of a date-time comes through here.
#[inline]
pub(super) fn push_zero_padded(dest: &mut String, value: u32, digits: u32) {
    let mut divider = 10u32.pow(digits - 1);

    while divider > 0 {
        dest.push((START_ZERO + (value / divider % 10) as u8) as char);
        divider /= 10;
    }
}
