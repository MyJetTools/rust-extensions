use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, NaiveDate, Utc};

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

    pub fn duration_since(&self, before: DateTimeAsMicroseconds) -> Duration {
        let dur = self.unix_microseconds - before.unix_microseconds;

        if dur < 0 {
            return Duration::from_micros(0);
        }

        Duration::from_micros(dur as u64)
    }

    pub fn to_rfc3339(&self) -> String {
        let chrono = self.to_chrono_utc();
        return chrono.to_rfc3339();
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
}
