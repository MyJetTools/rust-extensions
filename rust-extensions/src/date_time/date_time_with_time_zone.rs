use chrono::{Datelike, SecondsFormat, Timelike};
use serde::{Deserialize, Serialize};

use super::{DateTimeAsMicroseconds, DateTimeStruct, TimeStruct, TimeZone};

/// A UTC instant paired with a [`TimeZone`], so it can be rendered back into the
/// human-visible local wall-clock time it was captured in.
///
/// `date_time` stays UTC (same as a plain [`DateTimeAsMicroseconds`]); `time_zone` carries
/// the offset from UTC.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct DateTimeAsMicrosecondsWithTimeZone {
    /// The instant itself, kept in UTC.
    pub date_time: DateTimeAsMicroseconds,
    /// The timezone the instant should be rendered in.
    pub time_zone: TimeZone,
}

impl DateTimeAsMicrosecondsWithTimeZone {
    pub fn new(date_time: DateTimeAsMicroseconds, time_zone: TimeZone) -> Self {
        Self {
            date_time,
            time_zone,
        }
    }

    /// Builds the value from a `server_time` (UTC) and the `local_time` the same moment reads
    /// as on the local clock, deriving the [`TimeZone`] via
    /// [`TimeZone::from_server_and_local_time`] (rounded to the nearest 15 minutes). The
    /// stored `date_time` is the UTC `server_time`.
    pub fn from_server_and_local_time(
        server_time: DateTimeAsMicroseconds,
        local_time: DateTimeAsMicroseconds,
    ) -> Self {
        Self {
            date_time: server_time,
            time_zone: TimeZone::from_server_and_local_time(server_time, local_time),
        }
    }

    /// Parses an RFC 3339 string that carries an offset (`...+01:00` or `...Z`) into the
    /// UTC instant plus the [`TimeZone`]. `Z` is read as `UTC+0`.
    pub fn from_str(src: &str) -> Option<Self> {
        let parsed = chrono::DateTime::parse_from_rfc3339(src).ok()?;

        Some(Self {
            date_time: DateTimeAsMicroseconds::new(parsed.timestamp_micros()),
            // local_minus_utc() is in seconds; the offset is exact to the minute.
            time_zone: TimeZone::from_minutes(parsed.offset().local_minus_utc() / 60),
        })
    }

    /// The **local wall-clock time** (offset applied) as a [`DateTimeStruct`] — its
    /// `year`/`month`/`day`/`time`/`dow` are all the local values a person would read off a clock.
    pub fn to_local_date_time_struct(&self) -> DateTimeStruct {
        let local = self
            .date_time
            .to_chrono_utc()
            .with_timezone(&self.time_zone.to_fixed_offset());

        DateTimeStruct {
            year: local.year(),
            month: local.month(),
            day: local.day(),
            dow: Some(local.weekday()),
            time: TimeStruct {
                hour: local.hour(),
                min: local.minute(),
                sec: local.second(),
                micros: local.nanosecond() / 1000,
            },
        }
    }

    /// RFC 3339 with the numeric offset suffix and fixed microsecond precision, e.g.
    /// `2021-04-25T18:30:03.000000+01:00`. This is the serde wire format.
    pub fn to_rfc3339(&self) -> String {
        let local = self
            .date_time
            .to_chrono_utc()
            .with_timezone(&self.time_zone.to_fixed_offset());

        local.to_rfc3339_opts(SecondsFormat::Micros, false)
    }

    /// Human-readable local time without the zone suffix: `2021-04-25 18:30:03`.
    pub fn to_compact_string(&self) -> String {
        let dt = self.to_local_date_time_struct();
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            dt.year, dt.month, dt.day, dt.time.hour, dt.time.min, dt.time.sec
        )
    }
}

impl std::fmt::Debug for DateTimeAsMicrosecondsWithTimeZone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'", self.to_rfc3339())
    }
}

/// Serialized as a single RFC 3339 string with the numeric offset — the one string encodes
/// both the instant and the timezone (`2021-04-25T18:30:03.000000+01:00`).
impl Serialize for DateTimeAsMicrosecondsWithTimeZone {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.to_rfc3339().as_str())
    }
}

impl<'de> Deserialize<'de> for DateTimeAsMicrosecondsWithTimeZone {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(DateTimeAsMicrosecondsWithTimeZoneVisitor)
    }
}

struct DateTimeAsMicrosecondsWithTimeZoneVisitor;

impl<'de> serde::de::Visitor<'de> for DateTimeAsMicrosecondsWithTimeZoneVisitor {
    type Value = DateTimeAsMicrosecondsWithTimeZone;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("an RFC 3339 date-time string with an offset")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        match DateTimeAsMicrosecondsWithTimeZone::from_str(v) {
            Some(result) => Ok(result),
            None => Err(E::invalid_value(serde::de::Unexpected::Str(v), &self)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(src: &str) -> DateTimeAsMicroseconds {
        DateTimeAsMicroseconds::parse_iso_string(src).unwrap()
    }

    fn tz(minutes: i32) -> TimeZone {
        TimeZone::from_minutes(minutes)
    }

    #[test]
    fn local_struct_applies_positive_offset() {
        // 17:30 UTC at UTC+1 is 18:30 local.
        let value = DateTimeAsMicrosecondsWithTimeZone::new(utc("2021-04-25T17:30:03.000Z"), tz(60));

        let local = value.to_local_date_time_struct();
        assert_eq!(2021, local.year);
        assert_eq!(4, local.month);
        assert_eq!(25, local.day);
        assert_eq!(18, local.time.hour);
        assert_eq!(30, local.time.min);
        assert_eq!(3, local.time.sec);
    }

    #[test]
    fn local_struct_applies_negative_offset_across_midnight() {
        // 01:30 UTC at UTC-5 is 20:30 the previous day.
        let value =
            DateTimeAsMicrosecondsWithTimeZone::new(utc("2021-04-25T01:30:00.000Z"), tz(-300));

        let local = value.to_local_date_time_struct();
        assert_eq!(2021, local.year);
        assert_eq!(4, local.month);
        assert_eq!(24, local.day);
        assert_eq!(20, local.time.hour);
        assert_eq!(30, local.time.min);
    }

    #[test]
    fn to_rfc3339_renders_offset() {
        let value = DateTimeAsMicrosecondsWithTimeZone::new(utc("2021-04-25T17:30:03.000Z"), tz(60));
        assert_eq!("2021-04-25T18:30:03.000000+01:00", value.to_rfc3339());

        let value =
            DateTimeAsMicrosecondsWithTimeZone::new(utc("2021-04-25T17:30:03.000Z"), tz(-300));
        assert_eq!("2021-04-25T12:30:03.000000-05:00", value.to_rfc3339());

        // Half-hour offset (UTC+5:30 = 330 minutes).
        let value =
            DateTimeAsMicrosecondsWithTimeZone::new(utc("2021-04-25T17:30:03.000Z"), tz(330));
        assert_eq!("2021-04-25T23:00:03.000000+05:30", value.to_rfc3339());
    }

    #[test]
    fn to_compact_string_renders_local_time() {
        let value = DateTimeAsMicrosecondsWithTimeZone::new(utc("2021-04-25T17:30:03.000Z"), tz(60));
        assert_eq!("2021-04-25 18:30:03", value.to_compact_string());
    }

    #[test]
    fn from_str_reads_offset_into_minutes() {
        let value =
            DateTimeAsMicrosecondsWithTimeZone::from_str("2021-04-25T18:30:03+01:00").unwrap();

        assert_eq!(60, value.time_zone.offset_in_minutes());
        // The stored instant is UTC.
        assert_eq!(utc("2021-04-25T17:30:03.000Z"), value.date_time);
    }

    #[test]
    fn from_str_reads_z_as_utc() {
        let value = DateTimeAsMicrosecondsWithTimeZone::from_str("2021-04-25T17:30:03Z").unwrap();

        assert_eq!(0, value.time_zone.offset_in_minutes());
        assert_eq!(utc("2021-04-25T17:30:03.000Z"), value.date_time);
    }

    #[test]
    fn from_str_reads_half_hour_offset() {
        let value =
            DateTimeAsMicrosecondsWithTimeZone::from_str("2021-04-25T23:00:03+05:30").unwrap();

        assert_eq!(330, value.time_zone.offset_in_minutes());
        assert_eq!(utc("2021-04-25T17:30:03.000Z"), value.date_time);
    }

    #[test]
    fn from_server_and_local_time_derives_time_zone() {
        let server = utc("2021-04-25T17:30:03.000Z");

        let mut local = server;
        local.add_minutes(58); // rounds to +60

        let value = DateTimeAsMicrosecondsWithTimeZone::from_server_and_local_time(server, local);

        assert_eq!(60, value.time_zone.offset_in_minutes());
        assert_eq!(server, value.date_time);
        // Server 17:30 UTC rendered in the derived +1h zone reads 18:30 local.
        assert_eq!("2021-04-25 18:30:03", value.to_compact_string());
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestStruct {
        a: DateTimeAsMicrosecondsWithTimeZone,
    }

    #[test]
    fn serialize_produces_offset_string() {
        let value = TestStruct {
            a: DateTimeAsMicrosecondsWithTimeZone::new(utc("2021-04-25T17:30:03.000Z"), tz(60)),
        };

        let json = serde_json::to_string(&value).unwrap();
        assert_eq!("{\"a\":\"2021-04-25T18:30:03.000000+01:00\"}", json.as_str());
    }

    #[test]
    fn round_trip_keeps_instant_and_offset() {
        for (iso, tz_minutes) in [
            ("2021-04-25T17:30:03.123456Z", 60),
            ("2021-04-25T17:30:03.000Z", -300),
            ("2021-04-25T17:30:03.000Z", 330),
            ("1969-01-01T00:00:00.000Z", -60),
        ] {
            let value = TestStruct {
                a: DateTimeAsMicrosecondsWithTimeZone::new(utc(iso), tz(tz_minutes)),
            };

            let json = serde_json::to_string(&value).unwrap();
            let restored: TestStruct = serde_json::from_str(&json).unwrap();

            assert_eq!(value, restored, "iso {} tz {}", iso, tz_minutes);
        }
    }

    #[test]
    fn deserialize_garbage_is_an_error_not_a_panic() {
        for bad in [
            "{\"a\":\"\"}",
            "{\"a\":\"not-a-date\"}",
            "{\"a\":\"2021-04-25T17:30:03\"}", // no offset
            "{\"a\":1619371803000000}",       // a bare number is not accepted here
            "{\"a\":null}",
        ] {
            let result: Result<TestStruct, _> = serde_json::from_str(bad);
            assert!(result.is_err(), "expected an error for {}", bad);
        }
    }
}
