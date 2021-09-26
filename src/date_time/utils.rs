use std::time::{Duration, UNIX_EPOCH};

use chrono::{DateTime, Utc};

use chrono::NaiveDate;

const START_ZERO: u8 = '0' as u8;

pub fn parse_iso_string(src: &[u8]) -> Option<i64> {
    let year = (src[0] - START_ZERO) as i32 * 1000
        + (src[1] - START_ZERO) as i32 * 100
        + (src[2] - START_ZERO) as i32 * 10
        + (src[3] - START_ZERO) as i32;

    let month = (src[5] - START_ZERO) as u32 * 10 + (src[6] - START_ZERO) as u32;

    let day = (src[8] - START_ZERO) as u32 * 10 + (src[9] - START_ZERO) as u32;

    let hour = (src[11] - START_ZERO) as u32 * 10 + (src[12] - START_ZERO) as u32;

    let min = (src[14] - START_ZERO) as u32 * 10 + (src[15] - START_ZERO) as u32;

    let sec = (src[17] - START_ZERO) as u32 * 10 + (src[18] - START_ZERO) as u32;

    if src.len() <= 19 {
        let date_time = NaiveDate::from_ymd(year, month, day).and_hms_milli(hour, min, sec, 0);
        return Some(date_time.timestamp_millis() * 1000);
    }
    let msec = (src[20] - START_ZERO) as u32 * 100
        + (src[21] - START_ZERO) as u32 * 10
        + (src[22] - START_ZERO) as u32;

    let date_time = NaiveDate::from_ymd(year, month, day).and_hms_milli(hour, min, sec, msec);

    Some(date_time.timestamp_millis() * 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_parse_iso_string() {
        let src = "2021-04-25T17:30:43.605Z";
        let result = parse_iso_string(src.as_bytes());

        assert_eq!(1619371843605000, result.unwrap());
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
