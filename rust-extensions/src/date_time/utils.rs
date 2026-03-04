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
