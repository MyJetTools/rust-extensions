use std::{fmt::Debug, time::Duration};

use super::DateTimeAsMicroseconds;

pub enum DateTimeDuration {
    Positive(Duration),
    Negative(Duration),
    Zero,
}

impl DateTimeDuration {
    pub fn new(prev: &DateTimeAsMicroseconds, next: &DateTimeAsMicroseconds) -> Self {
        let dur = next.unix_microseconds - prev.unix_microseconds;

        if dur > 0 {
            return Self::Positive(Duration::from_micros(dur as u64));
        }

        if dur < 0 {
            let dur = -dur;
            return Self::Negative(Duration::from_micros(dur as u64));
        }

        Self::Zero
    }

    pub fn as_positive_or_zero(&self) -> Duration {
        match self {
            Self::Positive(duration) => *duration,
            _ => Duration::from_secs(0),
        }
    }

    pub fn as_negative_or_zero(&self) -> Duration {
        match self {
            Self::Negative(duration) => *duration,
            _ => Duration::from_secs(0),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Self::Positive(duration) => {
                format!("+{}", crate::duration_utils::duration_to_string(*duration))
            }
            Self::Negative(duration) => {
                format!("-{}", crate::duration_utils::duration_to_string(*duration))
            }
            Self::Zero => String::from("0"),
        }
    }

    pub fn get_full_micros(&self) -> i64 {
        match self {
            Self::Positive(duration) => duration.as_micros() as i64,
            Self::Negative(duration) => -(duration.as_micros() as i64),
            Self::Zero => 0,
        }
    }

    pub fn get_full_millis(&self) -> i64 {
        match self {
            Self::Positive(duration) => duration.as_millis() as i64,
            Self::Negative(duration) => -(duration.as_millis() as i64),
            Self::Zero => 0,
        }
    }

    pub fn get_full_seconds(&self) -> i64 {
        match self {
            Self::Positive(duration) => duration.as_secs() as i64,
            Self::Negative(duration) => -(duration.as_secs() as i64),
            Self::Zero => 0,
        }
    }

    pub fn get_full_minutes(&self) -> i64 {
        match self {
            Self::Positive(duration) => duration.as_secs() as i64 / 60,
            Self::Negative(duration) => -(duration.as_secs() as i64 / 60),
            Self::Zero => 0,
        }
    }

    pub fn get_full_hours(&self) -> i64 {
        match self {
            Self::Positive(duration) => duration.as_secs() as i64 / 3600,
            Self::Negative(duration) => -(duration.as_secs() as i64 / 3600),
            Self::Zero => 0,
        }
    }

    pub fn get_full_days(&self) -> i64 {
        match self {
            Self::Positive(duration) => duration.as_secs() as i64 / (3600 * 24),
            Self::Negative(duration) => -(duration.as_secs() as i64 / (3600 * 24)),
            Self::Zero => 0,
        }
    }
}

impl Debug for DateTimeDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Positive(duration) => {
                write!(
                    f,
                    "+{}",
                    crate::duration_utils::duration_to_string(*duration)
                )
            }
            Self::Negative(duration) => {
                write!(
                    f,
                    "-{}",
                    crate::duration_utils::duration_to_string(*duration)
                )
            }
            Self::Zero => write!(f, "0"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::date_time::DateTimeAsMicroseconds;

    use super::DateTimeDuration;

    #[test]
    fn test_positive_duration() {
        let now = DateTimeAsMicroseconds::now();
        let prev = DateTimeAsMicroseconds::new(now.unix_microseconds - 1);

        let duration = DateTimeDuration::new(&prev, &now);

        match duration {
            DateTimeDuration::Positive(value) => {
                assert_eq!(value.as_micros(), 1);
            }
            DateTimeDuration::Negative(_) => {
                panic!("Should not be here")
            }
            DateTimeDuration::Zero => {
                panic!("Should not be here")
            }
        }
    }

    #[test]
    fn test_negative_duration() {
        let now = DateTimeAsMicroseconds::now();
        let prev = DateTimeAsMicroseconds::new(now.unix_microseconds + 1);

        let duration = DateTimeDuration::new(&prev, &now);

        match duration {
            DateTimeDuration::Positive(_) => {
                panic!("Should not be here")
            }
            DateTimeDuration::Negative(value) => {
                assert_eq!(value.as_micros(), 1);
            }
            DateTimeDuration::Zero => {
                panic!("Should not be here")
            }
        }
    }

    #[test]
    fn test_zero_duration() {
        let now = DateTimeAsMicroseconds::now();
        let prev = DateTimeAsMicroseconds::new(now.unix_microseconds);

        let duration = DateTimeDuration::new(&prev, &now);

        match duration {
            DateTimeDuration::Positive(_) => {
                panic!("Should not be here")
            }
            DateTimeDuration::Negative(_) => {
                panic!("Should not be here")
            }
            DateTimeDuration::Zero => {
                println!("Zero");
            }
        }
    }

    #[test]
    fn test_get_full_seconds() {
        let after = DateTimeAsMicroseconds::from_str("2021-03-05T01:12:32").unwrap();

        let before = DateTimeAsMicroseconds::from_str("2021-03-05T01:12:31").unwrap();

        let duration = after - before;

        assert_eq!(duration.get_full_seconds(), 1);

        let duration = before - after;

        assert_eq!(duration.get_full_seconds(), -1);
    }

    #[test]
    fn test_get_full_minutes() {
        let after = DateTimeAsMicroseconds::from_str("2021-03-05T01:12:32").unwrap();

        let before = DateTimeAsMicroseconds::from_str("2021-03-05T01:12:31").unwrap();

        let duration = after - before;

        assert_eq!(duration.get_full_minutes(), 0);

        let duration = before - after;

        assert_eq!(duration.get_full_minutes(), 0);

        let before = DateTimeAsMicroseconds::from_str("2021-03-05T01:11:32").unwrap();

        let duration = after - before;

        assert_eq!(duration.get_full_minutes(), 1);

        let duration = before - after;

        assert_eq!(duration.get_full_minutes(), -1);
    }
}
