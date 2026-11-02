use std::sync::atomic::AtomicI64;

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
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn as_date_time(&self) -> DateTimeAsMicroseconds {
        let unix_microseconds = self
            .unix_microseconds
            .load(std::sync::atomic::Ordering::Relaxed);

        DateTimeAsMicroseconds::new(unix_microseconds)
    }

    pub fn now() -> Self {
        Self::new(DateTimeAsMicroseconds::now().unix_microseconds)
    }

    pub fn update(&self, value: DateTimeAsMicroseconds) {
        self.unix_microseconds.store(
            value.unix_microseconds,
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    pub fn parse_iso_string(iso_string: &str) -> Option<Self> {
        let dt = DateTimeStruct::parse_rfc3339_str(iso_string.as_bytes())?;
        return Some(Self::new(dt.to_unix_microseconds()?));
    }

    pub fn seconds_before(&self, before: DateTimeAsMicroseconds) -> i64 {
        (self.get_unix_microseconds() - before.unix_microseconds) / 1000000
    }

    pub fn duration_since(&self, before: DateTimeAsMicroseconds) -> DateTimeDuration {
        let current = self.as_date_time();
        DateTimeDuration::new(&before, &current)
    }

    pub fn to_rfc3339(&self) -> String {
        return self.as_date_time().to_rfc3339();
    }

    pub fn to_rfc5322(&self) -> String {
        let dt: DateTimeStruct = DateTimeAsMicroseconds::new(self.get_unix_microseconds()).into();
        return dt.to_rfc5322();
    }

    pub fn to_compact_date_time_string(&self) -> String {
        let dt: DateTimeStruct = DateTimeAsMicroseconds::new(self.get_unix_microseconds()).into();
        return dt.to_compact_date_time_string();
    }
}

impl Clone for AtomicDateTimeAsMicroseconds {
    fn clone(&self) -> Self {
        let value = self
            .unix_microseconds
            .load(std::sync::atomic::Ordering::Relaxed);
        Self {
            unix_microseconds: AtomicI64::new(value),
        }
    }
}

impl Into<AtomicDateTimeAsMicroseconds> for DateTimeAsMicroseconds {
    fn into(self) -> AtomicDateTimeAsMicroseconds {
        AtomicDateTimeAsMicroseconds::new(self.unix_microseconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A moment before 1970 used to go through `as u64` and panic.
    #[test]
    fn to_rfc3339_before_the_epoch() {
        let value = AtomicDateTimeAsMicroseconds::new(-1);

        assert_eq!("1969-12-31T23:59:59.999999+00:00", value.to_rfc3339());
    }
}
