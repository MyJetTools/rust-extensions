use std::time::Duration;

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
}
