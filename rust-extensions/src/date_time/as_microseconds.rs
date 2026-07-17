use std::{
    fmt::Display,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use super::{ClientServerTimeDifference, DateTimeDuration, DateTimeStruct};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DateTimeAsMicroseconds {
    pub unix_microseconds: i64,
}

impl DateTimeAsMicroseconds {
    pub fn new(unix_microseconds: i64) -> Self {
        Self { unix_microseconds }
    }

    pub fn from_nanos(value: i64) -> Self {
        Self {
            unix_microseconds: value / 1000,
        }
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
        SystemTime::now().into()
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

    /// Parses a **raw JSON value token**, exactly as a JSON parser hands it over: a quoted
    /// string keeps its quotes (`"2021-04-25T17:30:03.000000Z"`), a number arrives bare
    /// (`1619371803000000`). This is the single place that knows every spelling this type
    /// accepts on the wire, so serde and any external deserializer (my-json, my-http-utils)
    /// stay in agreement.
    ///
    /// | Token | Read as |
    /// |---|---|
    /// | `"2021-04-25T17:30:03.000000Z"` | RFC 3339, `Z` |
    /// | `"2021-04-25T17:30:03+00:00"` | RFC 3339, numeric zero offset (older `to_rfc3339()`) |
    /// | `"2021-04-25"` / `"2021-04-25T17:30"` | date, or date-time without seconds |
    /// | `"1619371803000000"` | digits in a string -> unix timestamp, unit sniffed by magnitude |
    /// | `1619371803000000` | bare number -> unix timestamp, unit sniffed by magnitude |
    /// | `null` | `None` |
    ///
    /// Quotes are stripped when present and the content is parsed the same way either way,
    /// so a number - quoted or bare - always goes through [`From<i64>`], which sniffs its unit
    /// (seconds / millis / micros / nanos) by magnitude. If you have already stripped the
    /// quotes yourself, calling [`Self::from_str`] directly is equivalent.
    ///
    /// Escapes are not resolved (no RFC 3339 spelling contains one), and single quotes are
    /// accepted alongside double ones, matching my-json's own tolerance.
    pub fn from_json_value_str(src: &str) -> Option<Self> {
        let src = src.trim();

        if src.is_empty() || src == "null" {
            return None;
        }

        // Quoted or bare, the content is parsed identically: from_str sniffs it itself -
        // an RFC 3339 date-time, or digits handed to `From<i64>` for unit detection.
        match strip_json_quotes(src) {
            Some(inner) => Self::from_str(inner),
            None => Self::from_str(src),
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

    /// RFC 3339 in UTC with a `Z` suffix and fixed microsecond precision:
    /// `2021-04-25T17:30:03.000000Z`. This is the serde wire format.
    ///
    /// Unlike [`Self::to_rfc3339`] (which renders the zero offset as `+00:00`),
    /// the width is fixed, so lexicographic order matches chronological order.
    pub fn to_rfc3339_utc(&self) -> String {
        let chrono = self.to_chrono_utc();
        return chrono.to_rfc3339_opts(SecondsFormat::Micros, true);
    }

    pub fn to_rfc5322(&self) -> String {
        let dt: DateTimeStruct = self.into();
        return dt.to_rfc5322();
    }

    pub fn to_rfc7231(&self) -> String {
        let dt: DateTimeStruct = self.into();
        return dt.to_rfc7231();
    }

    pub fn to_rfc2822(&self) -> String {
        let dt: DateTimeStruct = self.into();
        return dt.to_rfc2822();
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




/// Always writes RFC 3339 (`2021-04-25T17:30:03.000000Z`) - the format OpenAPI
/// `format: date-time` promises and the one every other participant already speaks.
impl Serialize for DateTimeAsMicroseconds {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.to_rfc3339_utc().as_str())
    }
}

/// Tolerant reader: accepts both the RFC 3339 string written above and the bare
/// unix-microseconds number written by the previous `#[serde(transparent)]` impl.
///
/// The number branch must stay, and must stay raw: historical payloads (MyNoSql
/// entities, settings files, Service Bus messages, jsonb columns) hold values like
/// `1704164645000000`, and a reader that rejects or reinterprets them makes already
/// persisted data unreadable - which no deploy rollback can undo.
///
/// This mirrors [`DateTimeAsMicroseconds::from_json_value_str`], which is the same
/// dispatch for deserializers that get a *raw* token. serde cannot call it directly -
/// it hands the visitor an already-decoded value, having consumed the quotes - so both
/// routes instead bottom out in the same primitives: `from_str` for a string, `From<i64>`
/// for a number. `serde_and_from_json_value_str_agree` pins the two together.
impl<'de> Deserialize<'de> for DateTimeAsMicroseconds {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(DateTimeAsMicrosecondsVisitor)
    }
}

struct DateTimeAsMicrosecondsVisitor;

impl<'de> serde::de::Visitor<'de> for DateTimeAsMicrosecondsVisitor {
    type Value = DateTimeAsMicroseconds;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("an RFC 3339 date-time string or a unix microseconds number")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        // The string branch of `from_json_value_str`. from_str sniffs the content itself:
        // an RFC 3339 date-time, or digits as a unix timestamp.
        match DateTimeAsMicroseconds::from_str(v) {
            Some(result) => Ok(result),
            None => Err(E::invalid_value(serde::de::Unexpected::Str(v), &self)),
        }
    }

    fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
        // The number branch of `from_json_value_str`: `From<i64>` sniffs the unit
        // (seconds / millis / micros / nanos) by magnitude.
        Ok(v.into())
    }

    fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
        match i64::try_from(v) {
            Ok(v) => Ok(v.into()),
            Err(_) => Err(E::invalid_value(serde::de::Unexpected::Unsigned(v), &self)),
        }
    }

    fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
        if !v.is_finite() || v < i64::MIN as f64 || v > i64::MAX as f64 {
            return Err(E::invalid_value(serde::de::Unexpected::Float(v), &self));
        }
        Ok((v as i64).into())
    }
}

/// Strips a single pair of matching surrounding quotes, marking the token as a JSON string.
/// `None` means the token was not quoted.
fn strip_json_quotes(src: &str) -> Option<&str> {
    let as_bytes = src.as_bytes();

    if as_bytes.len() < 2 {
        return None;
    }

    let first = as_bytes[0];
    let last = as_bytes[as_bytes.len() - 1];

    if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
        return Some(&src[1..src.len() - 1]);
    }

    None
}

impl Into<DateTimeAsMicroseconds> for SystemTime {
    fn into(self) -> DateTimeAsMicroseconds {
        let unix_microseconds = self.duration_since(UNIX_EPOCH).unwrap().as_micros() as i64;
        DateTimeAsMicroseconds { unix_microseconds }
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
        // Unit is detected by magnitude so that pre-1970 (negative) timestamps
        // resolve to the same unit as their positive counterparts.
        let magnitude = src.unsigned_abs();

        //Seconds up to [Mon Jan 01 2120 01:01:01]
        if magnitude < 4733514061 {
            return DateTimeAsMicroseconds::new(src * 1_000_000);
        }
        //Milliseconds up to [Mon Jan 01 2120 01:01:01]
        if magnitude < 4733514061000 {
            return DateTimeAsMicroseconds::new(src * 1000);
        }
        //Microseconds up to [Mon Jan 01 2120 01:01:01]
        if magnitude < 4733514061000000 {
            return DateTimeAsMicroseconds::new(src);
        }

        //Nanoseconds
        return DateTimeAsMicroseconds::new(src / 1000);
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
        //nanoseconds
        let value: DateTimeAsMicroseconds = 1679059876123456000i64.into();
        assert_eq!("2023-03-17T13:31:16.123456", &value.to_rfc3339()[..26]);

        //microseconds
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
    fn test_from_negative_timestamps() {
        // A negative timestamp is the same unit as its positive counterpart,
        // pointing before the unix epoch.

        //seconds
        let value: DateTimeAsMicroseconds = (-1679059876i64).into();
        assert_eq!(-1679059876_000000, value.unix_microseconds);

        //milliseconds
        let value: DateTimeAsMicroseconds = (-315525600123i64).into();
        assert_eq!(-315525600123_000, value.unix_microseconds);

        //microseconds
        let value: DateTimeAsMicroseconds = (-315525600123456i64).into();
        assert_eq!(-315525600123456, value.unix_microseconds);

        //nanoseconds
        let value: DateTimeAsMicroseconds = (-315525600123456000i64).into();
        assert_eq!(-315525600123456, value.unix_microseconds);
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

    /// Anything shorter than 5 bytes used to index past the end of the buffer and panic.
    /// Reachable from any JSON body once DateTime deserializes from a string.
    #[test]
    fn from_str_too_short_returns_none_instead_of_panicking() {
        for src in ["x", "ab", "abc", "2024-", "2021-04", "-", "T", "::"] {
            assert!(
                DateTimeAsMicroseconds::from_str(src).is_none(),
                "expected None for {}",
                src
            );
        }
    }

    #[test]
    fn from_str_garbage_returns_none() {
        for src in [
            "not-a-date",
            "hello world",
            "2021-04-25T",
            "----------",
            "@@@@@@@@@@@@@@@@@@@@",
        ] {
            assert!(
                DateTimeAsMicroseconds::from_str(src).is_none(),
                "expected None for {}",
                src
            );
        }
    }

    /// A truncated RFC 3339 value must not index past the end of the time part.
    #[test]
    fn from_str_truncated_rfc3339_returns_none_instead_of_panicking() {
        for src in [
            "2021-04-25T",
            "2021-04-25T1",
            "2021-04-25T17",
            "2021-04-25T17:",
            "2021-04-25T17:3",
            "2021-04-25T17:30:",
            "2021-04-25T17:30:0",
        ] {
            let _ = DateTimeAsMicroseconds::from_str(src);
        }
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestStruct {
        a: DateTimeAsMicroseconds,
    }

    // Mirrors the my-http-utils shape that broke: a DateTime nested inside an object.
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct NestedStruct {
        inner: TestStruct,
    }

    const SAMPLE_ISO: &str = "2021-04-25T17:30:03.000Z";
    const SAMPLE_MICROS: i64 = 1619371803000000;

    fn sample() -> DateTimeAsMicroseconds {
        DateTimeAsMicroseconds::parse_iso_string(SAMPLE_ISO).unwrap()
    }

    #[test]
    fn check_serialize() {
        let value = TestStruct { a: sample() };

        let json = serde_json::to_string(&value).unwrap();

        assert_eq!("{\"a\":\"2021-04-25T17:30:03.000000Z\"}", json.as_str());
    }

    #[test]
    fn deserialize_from_rfc3339_string() {
        let value: TestStruct =
            serde_json::from_str("{\"a\":\"2021-04-25T17:30:03.000000Z\"}").unwrap();

        assert_eq!(sample(), value.a);
    }

    /// The exact payload my-http-utils' client writer emits via `to_rfc3339()`.
    /// This is the case that used to fail with `invalid type: string ..., expected i64`.
    #[test]
    fn deserialize_from_rfc3339_string_with_numeric_offset() {
        let value: TestStruct =
            serde_json::from_str("{\"a\":\"2021-04-25T17:30:03+00:00\"}").unwrap();

        assert_eq!(sample(), value.a);
    }

    /// Historical data written by the previous `#[serde(transparent)]` impl must stay
    /// readable: a real microseconds timestamp is in the microseconds magnitude band,
    /// so `From<i64>` returns it unchanged.
    #[test]
    fn deserialize_from_legacy_unix_microseconds_number() {
        let value: TestStruct = serde_json::from_str("{\"a\":1619371803000000}").unwrap();

        assert_eq!(sample(), value.a);
        assert_eq!(SAMPLE_MICROS, value.a.unix_microseconds);
    }

    /// A number carries no unit, so it goes through `From<i64>`'s magnitude sniffing -
    /// the same rule the whole crate uses. Seconds, millis and micros land on one instant.
    #[test]
    fn deserialize_number_sniffs_the_unit() {
        let from_micros: TestStruct = serde_json::from_str("{\"a\":1619371803000000}").unwrap();
        let from_millis: TestStruct = serde_json::from_str("{\"a\":1619371803000}").unwrap();
        let from_seconds: TestStruct = serde_json::from_str("{\"a\":1619371803}").unwrap();

        assert_eq!(sample(), from_micros.a);
        assert_eq!(sample(), from_millis.a);
        assert_eq!(sample(), from_seconds.a);
    }

    #[test]
    fn deserialize_zero_number() {
        let value: TestStruct = serde_json::from_str("{\"a\":0}").unwrap();

        assert_eq!(0, value.a.unix_microseconds);
        assert_eq!("1970-01-01T00:00:00.000000Z", value.a.to_rfc3339_utc());
    }

    #[test]
    fn deserialize_from_negative_legacy_number() {
        let value: TestStruct = serde_json::from_str("{\"a\":-315525600123456}").unwrap();

        assert_eq!(-315525600123456, value.a.unix_microseconds);
    }

    #[test]
    fn deserialize_from_stringified_number() {
        let value: TestStruct = serde_json::from_str("{\"a\":\"1619371803000000\"}").unwrap();

        assert_eq!(sample(), value.a);
    }

    /// A bare numeric string is read as a unix timestamp, unit sniffed by magnitude -
    /// this is `from_str`'s long-standing contract, unchanged here.
    #[test]
    fn deserialize_stringified_number_sniffs_the_unit() {
        let from_seconds: TestStruct = serde_json::from_str("{\"a\":\"1619371803\"}").unwrap();
        assert_eq!(sample(), from_seconds.a);

        let from_millis: TestStruct = serde_json::from_str("{\"a\":\"1619371803000\"}").unwrap();
        assert_eq!(sample(), from_millis.a);
    }

    /// Both RFC 3339 renderings this codebase has ever produced must read back:
    /// the old `to_rfc3339()` (`+00:00` offset) and the new `to_rfc3339_utc()` (`Z`).
    #[test]
    fn deserialize_accepts_both_old_and_new_rfc3339_renderings() {
        for dt in [
            sample(),
            DateTimeAsMicroseconds::new(1619371803123456),
            DateTimeAsMicroseconds::new(1619371803000123),
            DateTimeAsMicroseconds::from_str("1969-01-01").unwrap(),
        ] {
            let old_format = dt.to_rfc3339();
            let new_format = dt.to_rfc3339_utc();

            let from_old: DateTimeAsMicroseconds =
                serde_json::from_str(&format!("\"{}\"", old_format)).unwrap();
            let from_new: DateTimeAsMicroseconds =
                serde_json::from_str(&format!("\"{}\"", new_format)).unwrap();

            assert_eq!(dt, from_old, "old to_rfc3339() rendering: {}", old_format);
            assert_eq!(dt, from_new, "new to_rfc3339_utc() rendering: {}", new_format);
        }
    }

    /// Trailing `Z` and a `+00:00` offset denote the same instant.
    #[test]
    fn deserialize_z_and_zero_offset_agree() {
        let with_z: DateTimeAsMicroseconds =
            serde_json::from_str("\"2021-04-25T17:30:03.605123Z\"").unwrap();
        let with_offset: DateTimeAsMicroseconds =
            serde_json::from_str("\"2021-04-25T17:30:03.605123+00:00\"").unwrap();

        assert_eq!(with_z, with_offset);
    }

    #[test]
    fn round_trip_serialize_deserialize() {
        let value = TestStruct { a: sample() };

        let json = serde_json::to_string(&value).unwrap();
        let restored: TestStruct = serde_json::from_str(&json).unwrap();

        assert_eq!(value, restored);
    }

    #[test]
    fn round_trip_keeps_microsecond_precision() {
        let value = TestStruct {
            a: DateTimeAsMicroseconds::new(1619371803123456),
        };

        let json = serde_json::to_string(&value).unwrap();
        assert_eq!("{\"a\":\"2021-04-25T17:30:03.123456Z\"}", json.as_str());

        let restored: TestStruct = serde_json::from_str(&json).unwrap();
        assert_eq!(1619371803123456, restored.a.unix_microseconds);
    }

    #[test]
    fn round_trip_negative_timestamp() {
        let value = TestStruct {
            a: DateTimeAsMicroseconds::from_str("1969-01-01").unwrap(),
        };

        let json = serde_json::to_string(&value).unwrap();
        let restored: TestStruct = serde_json::from_str(&json).unwrap();

        assert_eq!(value, restored);
    }

    #[test]
    fn round_trip_nested_object() {
        let value = NestedStruct {
            inner: TestStruct { a: sample() },
        };

        let json = serde_json::to_string(&value).unwrap();
        assert_eq!("{\"inner\":{\"a\":\"2021-04-25T17:30:03.000000Z\"}}", json);

        let restored: NestedStruct = serde_json::from_str(&json).unwrap();
        assert_eq!(value, restored);
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct OptionStruct {
        a: Option<DateTimeAsMicroseconds>,
    }

    #[test]
    fn option_some_round_trip() {
        let value = OptionStruct { a: Some(sample()) };

        let json = serde_json::to_string(&value).unwrap();
        assert_eq!("{\"a\":\"2021-04-25T17:30:03.000000Z\"}", json.as_str());

        let restored: OptionStruct = serde_json::from_str(&json).unwrap();
        assert_eq!(value, restored);
    }

    #[test]
    fn option_none_round_trip() {
        let value = OptionStruct { a: None };

        let json = serde_json::to_string(&value).unwrap();
        assert_eq!("{\"a\":null}", json.as_str());

        let restored: OptionStruct = serde_json::from_str(&json).unwrap();
        assert_eq!(value, restored);
    }

    #[test]
    fn option_reads_both_formats() {
        let from_number: OptionStruct = serde_json::from_str("{\"a\":1619371803000000}").unwrap();
        assert_eq!(Some(sample()), from_number.a);

        let from_string: OptionStruct =
            serde_json::from_str("{\"a\":\"2021-04-25T17:30:03.000000Z\"}").unwrap();
        assert_eq!(Some(sample()), from_string.a);
    }

    #[test]
    fn vec_round_trip() {
        let value = vec![sample(), sample().add(Duration::from_secs(1))];

        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(
            "[\"2021-04-25T17:30:03.000000Z\",\"2021-04-25T17:30:04.000000Z\"]",
            json.as_str()
        );

        let restored: Vec<DateTimeAsMicroseconds> = serde_json::from_str(&json).unwrap();
        assert_eq!(value, restored);
    }

    /// A tolerant reader has to cope with a collection mid-migration: old rows as
    /// numbers, new rows as strings, in the same payload.
    #[test]
    fn vec_reads_mixed_legacy_and_new_formats() {
        let restored: Vec<DateTimeAsMicroseconds> =
            serde_json::from_str("[\"2021-04-25T17:30:03.000000Z\",1619371803000000]").unwrap();

        assert_eq!(vec![sample(), sample()], restored);
    }

    #[test]
    fn from_json_value_str_reads_quoted_tokens() {
        for token in [
            "\"2021-04-25T17:30:03.000000Z\"",
            "\"2021-04-25T17:30:03+00:00\"",
            "\"2021-04-25T17:30:03Z\"",
            "\"1619371803000000\"",
            "\"1619371803000\"",
            "\"1619371803\"",
            "'2021-04-25T17:30:03.000000Z'",
        ] {
            assert_eq!(
                Some(sample()),
                DateTimeAsMicroseconds::from_json_value_str(token),
                "token {}",
                token
            );
        }
    }

    #[test]
    fn from_json_value_str_reads_bare_numbers() {
        for token in ["1619371803000000", "1619371803000", "1619371803"] {
            assert_eq!(
                Some(sample()),
                DateTimeAsMicroseconds::from_json_value_str(token),
                "token {}",
                token
            );
        }

        assert_eq!(
            -315525600123456,
            DateTimeAsMicroseconds::from_json_value_str("-315525600123456")
                .unwrap()
                .unix_microseconds
        );
    }

    /// Quotes are only packaging: strip them and the content parses identically, so the
    /// same digits mean the same instant either way.
    #[test]
    fn from_json_value_str_quotes_do_not_change_the_meaning() {
        for (quoted, bare) in [
            ("\"1619371803\"", "1619371803"),
            ("\"1619371803000\"", "1619371803000"),
            ("\"1619371803000000\"", "1619371803000000"),
            ("\"2021-04-25T17:30:03Z\"", "2021-04-25T17:30:03Z"),
        ] {
            assert_eq!(
                DateTimeAsMicroseconds::from_json_value_str(quoted),
                DateTimeAsMicroseconds::from_json_value_str(bare),
                "quoted {} vs bare {}",
                quoted,
                bare
            );
        }
    }

    #[test]
    fn from_json_value_str_handles_null_and_whitespace() {
        assert_eq!(None, DateTimeAsMicroseconds::from_json_value_str("null"));
        assert_eq!(None, DateTimeAsMicroseconds::from_json_value_str(""));
        assert_eq!(None, DateTimeAsMicroseconds::from_json_value_str("   "));
        assert_eq!(None, DateTimeAsMicroseconds::from_json_value_str("\"\""));

        assert_eq!(
            Some(sample()),
            DateTimeAsMicroseconds::from_json_value_str("  \"2021-04-25T17:30:03.000000Z\"  ")
        );
        assert_eq!(
            Some(sample()),
            DateTimeAsMicroseconds::from_json_value_str(" 1619371803000000 ")
        );
    }

    /// An unquoted date-time still reads - a parser that hands over bare tokens, or a
    /// query/path/header value.
    #[test]
    fn from_json_value_str_reads_unquoted_date_time() {
        assert_eq!(
            Some(sample()),
            DateTimeAsMicroseconds::from_json_value_str("2021-04-25T17:30:03.000000Z")
        );
    }

    #[test]
    fn from_json_value_str_rejects_garbage_without_panicking() {
        for token in [
            "\"x\"", "\"ab\"", "\"not-a-date\"", "x", "ab", "not-a-date", "\"", "'", "{}", "[]",
            "true",
        ] {
            assert_eq!(
                None,
                DateTimeAsMicroseconds::from_json_value_str(token),
                "token {}",
                token
            );
        }
    }

    /// The guarantee that matters: the serde route and the raw-token route never drift.
    /// For every spelling, feeding the token to serde_json and to `from_json_value_str`
    /// yields the same instant.
    #[test]
    fn serde_and_from_json_value_str_agree() {
        for token in [
            "\"2021-04-25T17:30:03.000000Z\"",
            "\"2021-04-25T17:30:03+00:00\"",
            "\"2021-04-25T17:30:03Z\"",
            "\"2021-04-25\"",
            "\"1619371803000000\"",
            "\"1619371803000\"",
            "\"1619371803\"",
            "1619371803000000",
            "1619371803000",
            "1619371803",
            "1000000",
            "0",
            "-315525600123456",
            "1679059876123456000",
        ] {
            let via_serde: DateTimeAsMicroseconds = serde_json::from_str(token)
                .unwrap_or_else(|e| panic!("serde failed on {}: {}", token, e));
            let via_raw = DateTimeAsMicroseconds::from_json_value_str(token)
                .unwrap_or_else(|| panic!("from_json_value_str returned None on {}", token));

            assert_eq!(via_serde, via_raw, "routes disagree on token {}", token);
        }
    }

    #[test]
    fn deserialize_garbage_string_is_an_error_not_a_panic() {
        for bad in [
            "{\"a\":\"\"}",
            "{\"a\":\"x\"}",
            "{\"a\":\"ab\"}",
            "{\"a\":\"not-a-date\"}",
            "{\"a\":\"2021-13-45T99:99:99Z\"}",
            "{\"a\":true}",
            "{\"a\":{}}",
            "{\"a\":[]}",
            "{\"a\":null}",
        ] {
            let result: Result<TestStruct, _> = serde_json::from_str(bad);
            assert!(result.is_err(), "expected an error for {}", bad);
        }
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
