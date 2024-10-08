use std::{
    fmt::Display,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::{ClientServerTimeDifference, DateTimeDuration, DateTimeStruct};

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
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
        let date_time = NaiveDate::from_ymd_opt(year, month, day).expect(&format!(
            "Invalid date with year:{}, month:{}, day:{}",
            year, month, day
        ));

        /* cSpell:disable */
        let date_time = date_time
            .and_hms_milli_opt(hour, minute, second, 0)
            .expect(&format!(
                "Invalid date with hour:{}, min:{}, sec:{}",
                hour, minute, second
            ));
        /* cSpell:enable */

        let result = date_time.and_utc().timestamp_millis() * 1000;

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
        if src == "" {
            return None;
        }

        if let Some(result) = DateTimeStruct::from_str(src) {
            return Some(Self::new(result.to_unix_microseconds()?));
        }

        let value: Result<i64, _> = src.parse();

        match value {
            Ok(result) => return Some(result.into()),
            Err(_) => return None,
        }
    }

    pub fn parse_iso_string(iso_string: &str) -> Option<Self> {
        DateTimeStruct::parse_rfc3339_str(iso_string.as_bytes())?.to_date_time_as_microseconds()
    }

    pub fn to_chrono_utc(&self) -> DateTime<Utc> {
        if self.unix_microseconds > 0 {
            let d = UNIX_EPOCH + Duration::from_micros(self.unix_microseconds as u64);
            DateTime::<Utc>::from(d)
        } else {
            let d = UNIX_EPOCH - Duration::from_micros(-self.unix_microseconds as u64);
            DateTime::<Utc>::from(d)
        }
    }

    pub fn seconds_before(&self, before: DateTimeAsMicroseconds) -> i64 {
        (self.unix_microseconds - before.unix_microseconds) / super::MICRO_SECONDS_IN_ONE_SECOND
    }

    pub fn add_seconds(&mut self, seconds: i64) {
        self.unix_microseconds += seconds * super::MICRO_SECONDS_IN_ONE_SECOND;
    }

    pub fn add_minutes(&mut self, minutes: i64) {
        self.unix_microseconds += minutes * super::MICRO_SECONDS_IN_ONE_MINUTE;
    }

    pub fn add_hours(&mut self, hour: i64) {
        self.unix_microseconds += hour * super::MICRO_SECONDS_IN_ONE_HOUR;
    }

    pub fn add_days(&mut self, days: i64) {
        self.unix_microseconds += days * super::MICRO_SECONDS_IN_ONE_DAY;
    }

    pub fn add(&self, duration: Duration) -> Self {
        Self {
            unix_microseconds: self.unix_microseconds + duration.as_micros() as i64,
        }
    }

    pub fn sub(&self, duration: Duration) -> Self {
        Self {
            unix_microseconds: self.unix_microseconds - duration.as_micros() as i64,
        }
    }

    pub fn duration_since(&self, before: DateTimeAsMicroseconds) -> DateTimeDuration {
        DateTimeDuration::new(&before, self)
    }

    pub fn to_rfc3339(&self) -> String {
        let chrono = self.to_chrono_utc();
        return chrono.to_rfc3339();
    }

    pub fn to_rfc5322(&self) -> String {
        let dt: DateTimeStruct = self.into();
        return dt.to_rfc5322();
    }

    pub fn to_rfc7231(&self) -> String {
        let dt: DateTimeStruct = self.into();
        return dt.to_rfc7231();
    }
    pub fn to_compact_date_time_string(&self) -> String {
        let dt: DateTimeStruct = self.into();
        return dt.to_compact_date_time_string();
    }

    pub fn get_client_server_time_difference(
        &self,
        server_time: DateTimeAsMicroseconds,
    ) -> ClientServerTimeDifference {
        ClientServerTimeDifference::new(*self, server_time)
    }

    pub fn is_later_than(&self, other_time: DateTimeAsMicroseconds) -> bool {
        self.unix_microseconds > other_time.unix_microseconds
    }

    pub fn is_earlier_than(&self, other_time: DateTimeAsMicroseconds) -> bool {
        self.unix_microseconds < other_time.unix_microseconds
    }

    pub fn client_input_time_to_server_time(
        client_input_time: DateTimeAsMicroseconds,
        client_now_time: DateTimeAsMicroseconds,
        server_now_time: DateTimeAsMicroseconds,
    ) -> Self {
        let difference = client_now_time.get_client_server_time_difference(server_now_time);

        let mut result = client_input_time.clone();
        result.add_minutes(-difference.difference_in_half_hours() * 30);
        result
    }
}

impl std::fmt::Debug for DateTimeAsMicroseconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'", self.to_rfc3339())
    }
}

impl Display for DateTimeAsMicroseconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.unix_microseconds)
    }
}
impl std::ops::Sub<DateTimeAsMicroseconds> for DateTimeAsMicroseconds {
    type Output = DateTimeDuration;

    fn sub(self, before: DateTimeAsMicroseconds) -> DateTimeDuration {
        self.duration_since(before)
    }
}

impl From<i64> for DateTimeAsMicroseconds {
    fn from(src: i64) -> Self {
        //Seconds946677600000
        if src < 4733514061 {
            return DateTimeAsMicroseconds::new(src * 1000_000);
        }
        //Milliseconds From  to [Mon Jan 01 2120 01:01:01]
        if src < 4733514061000 {
            return DateTimeAsMicroseconds::new(src * 1000);
        }

        return DateTimeAsMicroseconds::new(src);
    }
}

impl From<u64> for DateTimeAsMicroseconds {
    fn from(src: u64) -> Self {
        let src = src as i64;
        src.into()
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
    fn test_duration_since() {
        let now = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:03.000Z").unwrap();

        let before = DateTimeAsMicroseconds::new(now.unix_microseconds - 1);

        let duration = now.duration_since(before);

        assert_eq!(1, duration.as_positive_or_zero().as_micros());
    }

    #[test]
    fn test_from_trait() {
        let now = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:03.000Z").unwrap();

        let now_microseconds = now.unix_microseconds;

        let result: DateTimeAsMicroseconds = now_microseconds.into();

        assert_eq!("2021-04-25T17:30:03", &result.to_rfc3339()[..19]);

        let now_milliseconds = now_microseconds / 1000;

        let result: DateTimeAsMicroseconds = now_milliseconds.into();

        assert_eq!("2021-04-25T17:30:03", &result.to_rfc3339()[..19]);

        let now_seconds = now_milliseconds / 1000;

        let result: DateTimeAsMicroseconds = now_seconds.into();

        assert_eq!("2021-04-25T17:30:03", &result.to_rfc3339()[..19]);
    }

    #[test]
    fn test_parse_date_only() {
        let now = DateTimeAsMicroseconds::from_str("2021-04-25").unwrap();
        assert_eq!("2021-04-25T00:00:00", &now.to_rfc3339()[..19]);
    }

    #[test]
    fn client_input_time_to_server_time_1() {
        let client_time =
            DateTimeAsMicroseconds::parse_iso_string("2021-04-25T19:30:03.000Z").unwrap();
        let server_time =
            DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:00.000Z").unwrap();

        let input_time = DateTimeAsMicroseconds::client_input_time_to_server_time(
            DateTimeAsMicroseconds::parse_iso_string("2021-04-25T19:30:03.000Z").unwrap(),
            client_time,
            server_time,
        );
        assert_eq!("2021-04-25T17:30:03", &input_time.to_rfc3339()[..19]);
    }

    #[test]
    fn client_input_time_to_server_time_2() {
        let client_time =
            DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:03.000Z").unwrap();
        let server_time =
            DateTimeAsMicroseconds::parse_iso_string("2021-04-25T19:30:00.000Z").unwrap();

        let input_time = DateTimeAsMicroseconds::client_input_time_to_server_time(
            DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:03.000Z").unwrap(),
            client_time,
            server_time,
        );
        assert_eq!("2021-04-25T19:30:03", &input_time.to_rfc3339()[..19]);
    }

    #[test]
    fn client_input_time_to_server_time_4() {
        let client_time =
            DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:03.000Z").unwrap();
        let server_time =
            DateTimeAsMicroseconds::parse_iso_string("2021-04-25T19:30:00.000Z").unwrap();

        let input_time = DateTimeAsMicroseconds::client_input_time_to_server_time(
            DateTimeAsMicroseconds::parse_iso_string("2021-04-25T16:30:03.000Z").unwrap(),
            client_time,
            server_time,
        );
        assert_eq!("2021-04-25T18:30:03", &input_time.to_rfc3339()[..19]);
    }

    #[test]
    fn test_add_duration() {
        let dt = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:03.001Z").unwrap();

        assert_eq!(
            "2021-04-25T17:30:04.001",
            &dt.add(Duration::from_secs(1)).to_rfc3339()[0..23]
        );
    }

    #[test]
    fn test_sub_duration() {
        let dt = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:03.001Z").unwrap();

        assert_eq!(
            "2021-04-25T17:30:02.001",
            &dt.sub(Duration::from_secs(1)).to_rfc3339()[0..23]
        );
    }

    #[test]
    fn test_from_date() {
        let dt = DateTimeAsMicroseconds::from_str("2021-04-25").unwrap();

        assert_eq!("2021-04-25T00:00:00", &dt.to_rfc3339()[0..19]);
    }

    #[test]
    fn test_from_date_time_without_seconds() {
        let dt = DateTimeAsMicroseconds::from_str("2021-04-25T12:34").unwrap();

        assert_eq!("2021-04-25T12:34:00", &dt.to_rfc3339()[0..19]);
    }

    #[test]
    fn test_is_later_than() {
        let now = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:03.000Z").unwrap();
        let later = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:04.000Z").unwrap();
        let earlier = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:02.000Z").unwrap();

        assert!(later.is_later_than(now));
        assert!(!now.is_later_than(later));
        assert!(!now.is_later_than(now));
        assert!(!earlier.is_later_than(now));

        assert!(earlier.is_earlier_than(now));
    }

    #[test]
    fn test_from_mil_micro_secs() {
        //milliseconds
        let value: DateTimeAsMicroseconds = 1679059876123456i64.into();
        assert_eq!("2023-03-17T13:31:16.123456", &value.to_rfc3339()[..26]);
        let value: DateTimeAsMicroseconds = 315525600123456i64.into();
        assert_eq!("1979-12-31T22:00:00.123456", &value.to_rfc3339()[..26]);

        //milliseconds
        let value: DateTimeAsMicroseconds = 1679059876123i64.into();
        assert_eq!("2023-03-17T13:31:16.123", &value.to_rfc3339()[..23]);
        let value: DateTimeAsMicroseconds = 315525600123i64.into();
        assert_eq!("1979-12-31T22:00:00.123", &value.to_rfc3339()[..23]);

        //seconds
        let value: DateTimeAsMicroseconds = 1679059876i64.into();
        assert_eq!("2023-03-17T13:31:16", &value.to_rfc3339()[..19]);
        let value: DateTimeAsMicroseconds = 315525600i64.into();
        assert_eq!("1979-12-31T22:00:00", &value.to_rfc3339()[..19]);
    }

    #[test]
    fn some_random_tests() {
        let value: DateTimeAsMicroseconds = 1679000370000i64.into();
        println!("{}", value.to_rfc3339());
    }

    #[test]
    fn from_str_empty() {
        let time = DateTimeAsMicroseconds::from_str("");

        assert_eq!(true, time.is_none());
    }

    #[derive(Serialize)]
    struct TestStruct {
        a: DateTimeAsMicroseconds,
    }

    #[test]
    fn check_serialize() {
        let value = TestStruct {
            a: DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:03.000Z").unwrap(),
        };

        let json = serde_json::to_string(&value).unwrap();

        println!("{}", json);

        //assert_eq!("{\"a\":\"2021-04-25T17:30:03.000000Z\"}", json.as_str());
    }

    #[test]
    fn test_negative_date() {
        let b = DateTimeAsMicroseconds::from_str("1969-01-01").unwrap();
        assert_eq!("1969-01-01T00:00:00", &b.to_rfc3339()[..19]);
    }

    #[test]
    fn test_add() {
        let b = DateTimeAsMicroseconds::from_str("1969-01-01").unwrap();
        let b = b.add(Duration::from_secs(1));

        assert_eq!("1969-01-01T00:00:01", &b.to_rfc3339()[..19]);
    }

    #[test]
    fn test_greater() {
        let a = DateTimeAsMicroseconds::from_str("1969-01-01").unwrap();
        let b = DateTimeAsMicroseconds::from_str("1969-01-02").unwrap();

        assert!(a < b);
    }
}
