use std::time::{Duration, UNIX_EPOCH};

use chrono::{DateTime, Utc};

use chrono::NaiveDate;

const START_ZERO: u8 = '0' as u8;
const START_NINE: u8 = '9' as u8;

pub fn parse_iso_string(src: &[u8]) -> Option<i64> {
    let year = parse_four_digits(&src[0..4])?;

    let month = parse_two_digits(&src[5..7])?;

    let day = parse_two_digits(&src[8..10])?;

    let hour = parse_two_digits(&src[11..13])?;

    let min = parse_two_digits(&src[14..16])?;

    let sec = parse_two_digits(&src[17..19])?;

    let date_time =
        NaiveDate::from_ymd_opt(year, month, day)?.and_hms_milli_opt(hour, min, sec, 0)?;

    let result = date_time.timestamp_millis() * 1000;

    if src.len() <= 19 {
        return Some(result);
    }
    let microsec = parse_microseconds(&src[20..]);

    Some(result + microsec)
}

pub fn parse_url_encoded_iso_string(src: &[u8]) -> Option<i64> {
    let year = parse_four_digits(&src[0..4])?;

    let month = parse_two_digits(&src[5..7])?;

    let day = parse_two_digits(&src[8..10])?;

    let hour = parse_two_digits(&src[11..13])?;

    let min = parse_two_digits(&src[16..18])?;

    let sec = parse_two_digits(&src[21..23])?;

    let date_time =
        NaiveDate::from_ymd_opt(year, month, day)?.and_hms_milli_opt(hour, min, sec, 0)?;

    let result = date_time.timestamp_millis() * 1000;

    if src.len() <= 23 {
        return Some(result);
    }

    let microsec = parse_microseconds(&src[24..]);

    Some(result + microsec)
}

pub fn parse_compact_date_time(src: &[u8]) -> Option<i64> {
    let year = parse_four_digits(&src[0..4])?;

    let month = parse_two_digits(&src[4..6])?;

    let day = parse_two_digits(&src[6..8])?;

    let hour = parse_two_digits(&src[8..10])?;

    let min = parse_two_digits(&src[10..12])?;

    let sec = parse_two_digits(&src[12..14])?;

    let date_time =
        NaiveDate::from_ymd_opt(year, month, day)?.and_hms_milli_opt(hour, min, sec, 0)?;

    let result = date_time.timestamp_millis() * 1000;

    Some(result)
}

#[inline]
fn parse_number(src: u8) -> Option<i32> {
    if src < START_ZERO || src > START_NINE {
        return None;
    }

    return Some((src - START_ZERO) as i32);
}

#[inline]
fn parse_four_digits(src: &[u8]) -> Option<i32> {
    let result = parse_number(src[0])? * 1000
        + parse_number(src[1])? * 100
        + parse_number(src[2])? * 10
        + parse_number(src[3])?;

    Some(result)
}

#[inline]
fn parse_two_digits(src: &[u8]) -> Option<u32> {
    let result = parse_number(*src.get(0)?)? * 10 + parse_number(*src.get(1)?)?;
    Some(result as u32)
}

#[inline]
fn parse_microseconds(src: &[u8]) -> i64 {
    let mut multiplier: i32 = 100000;

    let mut result: i32 = 0;

    for b in src {
        let b = parse_number(*b);

        if b.is_none() {
            return result as i64;
        }

        result += b.unwrap() * multiplier;

        if multiplier == 1 {
            break;
        }

        multiplier /= 10;
    }

    result as i64
}

#[cfg(test)]
mod tests {
    use crate::date_time::DateTimeAsMicroseconds;

    use super::*;

    #[test]
    pub fn test_parse_iso_string() {
        let src = "2021-04-25T17:30:43.605Z";
        let result = parse_iso_string(src.as_bytes());

        assert_eq!(1619371843605000, result.unwrap());
    }

    #[test]
    pub fn test_parse_url_encoded_iso_string() {
        let src = "2021-04-25T17%3A30%3A43.605Z";
        let result = parse_url_encoded_iso_string(src.as_bytes());

        assert_eq!(1619371843605000, result.unwrap());
    }

    #[test]
    pub fn test_parse_iso_string_and_back() {
        let src = "2021-04-25T17:30:43.602432";
        let micros = parse_iso_string(src.as_bytes()).unwrap();

        let dt = DateTimeAsMicroseconds::new(micros);

        let dest = dt.to_rfc3339();

        assert_eq!(src, &dest[0..26]);
    }

    #[test]
    pub fn test_parse_url_endcoded_iso_string_and_back() {
        let src = "2021-04-25T17%3A30%3A43.602432";
        let micros = parse_url_encoded_iso_string(src.as_bytes()).unwrap();

        let dt = DateTimeAsMicroseconds::new(micros);

        let dest = dt.to_rfc3339();

        assert_eq!("2021-04-25T17:30:43.602432", &dest[0..26]);
    }

    #[test]
    pub fn test_parse_compact_string() {
        let src = "20210425173043";
        let micros = parse_compact_date_time(src.as_bytes()).unwrap();

        let dt = DateTimeAsMicroseconds::new(micros);

        let dest = dt.to_rfc3339();

        assert_eq!("2021-04-25T17:30:43", &dest[..19]);
    }

    #[test]
    pub fn test_parse_microseconds() {
        let src = "605Z";
        let result = parse_microseconds(src.as_bytes());

        assert_eq!(605000, result);
    }

    #[test]
    pub fn test_parse_microseconds_full_case() {
        let src = "605243";
        let result = parse_microseconds(src.as_bytes());

        assert_eq!(605243, result);
    }
}

pub trait ToUtcDateTime {
    fn to_utc_date_time(&self) -> DateTime<Utc>;
}

impl ToUtcDateTime for i64 {
    fn to_utc_date_time(&self) -> DateTime<Utc> {
        let d = UNIX_EPOCH + Duration::from_millis(*self as u64);

        DateTime::<Utc>::from(d)
    }
}
