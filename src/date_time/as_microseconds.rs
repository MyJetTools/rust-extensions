use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, NaiveDate, Utc};

use super::DateTimeDuration;

const ONE_SECOND: i64 = 1_000_000;

#[derive(Clone, Copy, Debug)]
pub struct DateTimeAsMicroseconds {
    pub unix_microseconds: i64,
}

impl DateTimeAsMicroseconds {
    pub fn new(unix_microseconds: i64) -> Self {
        Self { unix_microseconds }
    }

    pub fn create(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
        microsecond: i64,
    ) -> Self {
        let date_time =
            NaiveDate::from_ymd(year, month, day).and_hms_milli(hour, minute, second, 0);

        let result = date_time.timestamp_millis() * 1000;

        Self {
            unix_microseconds: result + microsecond,
        }
    }

    pub fn now() -> Self {
        let unix_microseconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as i64;

        Self { unix_microseconds }
    }

    pub fn from_str(src: &str) -> Option<Self> {
        let as_bytes = src.as_bytes();
        if as_bytes.len() == 14 {
            let result = super::utils::parse_compact_date_time(as_bytes)?;
            return DateTimeAsMicroseconds::new(result).into();
        }

        if as_bytes[4] == b'-' && as_bytes.len() >= 19 {
            if as_bytes[13] == b'%' {
                let result = super::utils::parse_url_encoded_iso_string(as_bytes)?;
                return DateTimeAsMicroseconds::new(result).into();
            } else {
                let result = super::utils::parse_iso_string(as_bytes)?;
                return DateTimeAsMicroseconds::new(result).into();
            }
        }

        let value: Result<i64, _> = src.parse();

        match value {
            Ok(result) => return Some(result.into()),
            Err(_) => return None,
        }
    }

    pub fn parse_iso_string(iso_string: &str) -> Option<Self> {
        let result = super::utils::parse_iso_string(iso_string.as_bytes())?;
        return Some(Self::new(result));
    }

    pub fn to_chrono_utc(&self) -> DateTime<Utc> {
        let d = UNIX_EPOCH + Duration::from_micros(self.unix_microseconds as u64);
        return DateTime::<Utc>::from(d);
    }

    pub fn seconds_before(&self, before: DateTimeAsMicroseconds) -> i64 {
        (self.unix_microseconds - before.unix_microseconds) / ONE_SECOND
    }

    pub fn add_seconds(&mut self, seconds: i64) {
        self.unix_microseconds += seconds * ONE_SECOND;
    }

    pub fn add_minutes(&mut self, minutes: i64) {
        self.add_seconds(60 * minutes);
    }

    pub fn add_hours(&mut self, hours: i64) {
        self.add_minutes(60 * hours);
    }

    pub fn add_days(&mut self, days: i64) {
        self.add_hours(24 * days);
    }

    pub fn duration_since(&self, before: DateTimeAsMicroseconds) -> DateTimeDuration {
        DateTimeDuration::new(&before, self)
    }

    pub fn to_rfc3339(&self) -> String {
        let chrono = self.to_chrono_utc();
        return chrono.to_rfc3339();
    }
}

impl From<i64> for DateTimeAsMicroseconds {
    fn from(src: i64) -> Self {
        //Milicecondes
        if src > 1577840461000 && src < 4733514061000 {
            return DateTimeAsMicroseconds::new(src * 1000);
        }
        //Seconds
        if src > 1577840461 && src < 4733514061 {
            return DateTimeAsMicroseconds::new(src * 1000_000);
        }

        return DateTimeAsMicroseconds::new(src);
    }
}

impl From<u64> for DateTimeAsMicroseconds {
    fn from(src: u64) -> Self {
        let src = src as i64;

        //Miliseconds
        if src > 1577840461000 && src < 4733514061000 {
            return DateTimeAsMicroseconds::new(src * 1000);
        }
        //Seconds
        if src > 1577840461 && src < 4733514061 {
            return DateTimeAsMicroseconds::new(src * 1000_000);
        }

        return DateTimeAsMicroseconds::new(src);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seconds_between() {
        let now = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:03.000Z").unwrap();
        let before = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:00.000Z").unwrap();
        assert_eq!(3, now.seconds_before(before));
    }

    #[test]
    fn test_add_seconds() {
        let mut now = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:03.000Z").unwrap();

        now.add_seconds(1);

        let result = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:04.000Z").unwrap();
        assert_eq!(now.unix_microseconds, result.unix_microseconds)
    }

    #[test]
    fn test_add_minutes() {
        let mut now = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:03.000Z").unwrap();

        now.add_minutes(1);

        let result = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:31:03.000Z").unwrap();
        assert_eq!(now.unix_microseconds, result.unix_microseconds)
    }

    #[test]
    fn test_add_hours() {
        let mut now = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:03.000Z").unwrap();

        now.add_hours(1);

        let result = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T18:30:03.000Z").unwrap();
        assert_eq!(now.unix_microseconds, result.unix_microseconds)
    }

    #[test]
    fn test_add_days() {
        let mut now = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:03.000Z").unwrap();

        now.add_days(1);

        let result = DateTimeAsMicroseconds::parse_iso_string("2021-04-26T17:30:03.000Z").unwrap();
        assert_eq!(now.unix_microseconds, result.unix_microseconds)
    }

    #[test]
    fn test_from_trait() {
        let now = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:03.000Z").unwrap();

        let now_microseconds = now.unix_microseconds;

        let result: DateTimeAsMicroseconds = now_microseconds.into();

        assert_eq!("2021-04-25T17:30:03", &result.to_rfc3339()[..19]);

        let now_miliseconds = now_microseconds / 1000;

        let result: DateTimeAsMicroseconds = now_miliseconds.into();

        assert_eq!("2021-04-25T17:30:03", &result.to_rfc3339()[..19]);

        let now_seconds = now_miliseconds / 1000;

        let result: DateTimeAsMicroseconds = now_seconds.into();

        assert_eq!("2021-04-25T17:30:03", &result.to_rfc3339()[..19]);
    }
}
