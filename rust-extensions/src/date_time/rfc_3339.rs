use std::time::{Duration, UNIX_EPOCH};

use chrono::{DateTime, Utc};

use chrono::NaiveDate;

use super::{DateTimeAsMicroseconds, DateTimeStruct, TimeStruct};

impl DateTimeStruct {
    pub fn parse_rfc3339_str(src: &[u8]) -> Option<Self> {
        let year = super::utils::parse_four_digits(&src[0..4])? as i32;

        let month = super::utils::parse_two_digits(&src[5..7])?;

        let day = super::utils::parse_two_digits(&src[8..10])?;

        let time = if src.len() > 10 {
            TimeStruct::parse_rfc_3339_time(&src[11..])?
        } else {
            TimeStruct::default()
        };

        return Some(Self {
            year,
            month,
            day,
            time,
            dow: None,
        });
    }

    pub fn parse_rfc3339_url_encoded_str(src: &[u8]) -> Option<Self> {
        let year = super::utils::parse_four_digits(&src[0..4])?;

        let month = super::utils::parse_two_digits(&src[5..7])?;

        let day = super::utils::parse_two_digits(&src[8..10])?;

        let time = if src.len() > 10 {
            TimeStruct::parse_rfc_3339_url_encoded_time(&src[11..])?
        } else {
            TimeStruct::default()
        };

        return Some(Self {
            year,
            month,
            day,
            time,
            dow: None,
        });
    }

    pub fn to_unix_microseconds(&self) -> Option<i64> {
        /* cSpell:disable */
        let date_time = NaiveDate::from_ymd_opt(self.year, self.month, self.day)?
            .and_hms_micro_opt(
                self.time.hour,
                self.time.min,
                self.time.sec,
                self.time.micros,
            )?;
        /* cSpell:enable */

        Some(date_time.and_utc().timestamp_micros())
    }

    pub fn to_date_time_as_microseconds(&self) -> Option<DateTimeAsMicroseconds> {
        let unix_microseconds = self.to_unix_microseconds()?;
        Some(DateTimeAsMicroseconds::new(unix_microseconds))
    }
}

impl<'s> TryInto<DateTimeAsMicroseconds> for &'s DateTimeStruct {
    type Error = ();

    fn try_into(self) -> Result<DateTimeAsMicroseconds, Self::Error> {
        match self.to_date_time_as_microseconds() {
            Some(result) => Ok(result),
            None => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::date_time::DateTimeAsMicroseconds;

    use super::*;

    #[test]
    pub fn test_parse_iso_string() {
        let src = "2021-04-25T17:30:43.605Z";
        let dt = DateTimeStruct::parse_rfc3339_str(src.as_bytes()).unwrap();

        assert_eq!(2021, dt.year);
        assert_eq!(4, dt.month);
        assert_eq!(25, dt.day);

        assert_eq!(17, dt.time.hour);
        assert_eq!(30, dt.time.min);
        assert_eq!(43, dt.time.sec);

        assert_eq!(605000, dt.time.micros);

        let result: DateTimeAsMicroseconds = dt.try_into().unwrap();

        assert_eq!(1619371843605000, result.unix_microseconds);
    }

    #[test]
    pub fn test_parse_url_encoded_iso_string() {
        let src = "2021-04-25T17%3A30%3A43.605Z";
        let dt = DateTimeStruct::parse_rfc3339_url_encoded_str(src.as_bytes()).unwrap();

        let result: DateTimeAsMicroseconds = dt.try_into().unwrap();

        assert_eq!(1619371843605000, result.unix_microseconds);
    }

    #[test]
    pub fn test_parse_iso_string_and_back() {
        let src = "2021-04-25T17:30:43.602432";
        let dt = DateTimeStruct::parse_rfc3339_str(src.as_bytes()).unwrap();

        assert_eq!(2021, dt.year);
        assert_eq!(4, dt.month);
        assert_eq!(25, dt.day);

        assert_eq!(17, dt.time.hour);
        assert_eq!(30, dt.time.min);
        assert_eq!(43, dt.time.sec);

        assert_eq!(602432, dt.time.micros);

        let result: DateTimeAsMicroseconds = dt.try_into().unwrap();

        let dest = result.to_rfc3339();

        assert_eq!(src, &dest[0..26]);
    }

    #[test]
    pub fn test_parse_url_encoded_iso_string_and_back() {
        let src = "2021-04-25T17%3A30%3A43.602432";
        let dt = DateTimeStruct::parse_rfc3339_url_encoded_str(src.as_bytes()).unwrap();

        let dt: DateTimeAsMicroseconds = dt.try_into().unwrap();

        let dest = dt.to_rfc3339();

        assert_eq!("2021-04-25T17:30:43.602432", &dest[0..26]);
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
