use std::{
    sync::atomic::AtomicI64,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Utc};

use super::{DateTimeAsMicroseconds, DateTimeDuration, DateTimeStruct};

#[derive(Debug)]
pub struct AtomicDateTimeAsMicroseconds {
    unix_microseconds: AtomicI64,
}

impl AtomicDateTimeAsMicroseconds {
    pub fn new(unix_microseconds: i64) -> Self {
        Self {
            unix_microseconds: AtomicI64::new(unix_microseconds),
        }
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
            Ok(result) => return Some(Self::new(result)),
            Err(_) => return None,
        }
    }

    pub fn get_unix_microseconds(&self) -> i64 {
        self.unix_microseconds
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn as_date_time(&self) -> DateTimeAsMicroseconds {
        let unix_microseconds = self
            .unix_microseconds
            .load(std::sync::atomic::Ordering::SeqCst);

        DateTimeAsMicroseconds::new(unix_microseconds)
    }

    pub fn now() -> Self {
        let unix_microseconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as i64;

        Self {
            unix_microseconds: AtomicI64::new(unix_microseconds),
        }
    }

    pub fn update(&self, value: DateTimeAsMicroseconds) {
        self.unix_microseconds
            .store(value.unix_microseconds, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn parse_iso_string(iso_string: &str) -> Option<Self> {
        let dt = DateTimeStruct::parse_rfc3339_str(iso_string.as_bytes())?;
        return Some(Self::new(dt.to_unix_microseconds()?));
    }

    pub fn to_chrono_utc(&self) -> DateTime<Utc> {
        let d = UNIX_EPOCH + Duration::from_micros(self.get_unix_microseconds() as u64);
        return DateTime::<Utc>::from(d);
    }

    pub fn seconds_before(&self, before: DateTimeAsMicroseconds) -> i64 {
        (self.get_unix_microseconds() - before.unix_microseconds) / 1000000
    }

    pub fn duration_since(&self, before: DateTimeAsMicroseconds) -> DateTimeDuration {
        let current = self.as_date_time();
        DateTimeDuration::new(&before, &current)
    }

    pub fn to_rfc3339(&self) -> String {
        let chrono = self.to_chrono_utc();
        return chrono.to_rfc3339();
    }

    pub fn to_rfc5322(&self) -> String {
        let dt: DateTimeStruct = DateTimeAsMicroseconds::new(self.get_unix_microseconds()).into();
        return dt.to_rfc5322();
    }
}
