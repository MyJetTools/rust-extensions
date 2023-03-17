use std::{num::ParseIntError, time::Duration};

pub trait DurationExtensions {
    fn from_str(s: &str) -> Result<Duration, ParseDurationError>;
    fn format_to_string(&self) -> String;
}

impl DurationExtensions for Duration {
    fn from_str(src: &str) -> Result<Duration, ParseDurationError> {
        parse_duration(src)
    }

    fn format_to_string(&self) -> String {
        duration_to_string(*self)
    }
}

pub fn parse_duration(src: &str) -> Result<Duration, ParseDurationError> {
    let secs = parse_seconds(src);

    match secs {
        Ok(secs) => Ok(Duration::from_secs(secs)),
        Err(_) => Err(ParseDurationError::Other(format!(
            "Invalid duration string '{}'",
            src
        ))),
    }
}

fn parse_seconds(str: &str) -> Result<u64, ParseDurationError> {
    let mut days: u64 = 0;
    let mut seconds = 0;
    for s in str.split_whitespace() {
        if s.ends_with("d") {
            let s = &s[..s.len() - 1];
            match s.parse() {
                Ok(result) => {
                    days = result;
                }
                Err(err) => return Err(ParseDurationError::ParseIntError(err)),
            }
        } else {
            seconds = parse_hms(s)?;
        }
    }

    Ok(days * 86400 + seconds)
}

fn parse_hms(src: &str) -> Result<u64, ParseDurationError> {
    let hms: Vec<&str> = src.split(":").collect();

    if hms.len() == 3 {
        let h = hms[0].parse::<u64>()?;
        let m = hms[1].parse::<u64>()?;
        let s = hms[2].parse::<u64>()?;
        let seconds = h * 3600 + m * 60 + s;
        return Ok(seconds);
    }

    if hms.len() == 2 {
        let m = hms[0].parse::<u64>()?;
        let s = hms[1].parse::<u64>()?;

        let seconds = m * 60 + s;
        return Ok(seconds);
    }

    let err =
        ParseDurationError::Other(format!("Can not parse hours minutes and seconds: {}", src));
    Err(err)
}

pub fn duration_to_string(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        return format!("{:?}", d);
    }

    return format_duration_seconds(secs as i64);
}

const SECS_PER_DAY: i64 = 3600 * 24;

fn format_duration_seconds(secs: i64) -> String {
    let mut secs = secs;
    let days = secs / SECS_PER_DAY;

    secs = secs - days * SECS_PER_DAY;

    let hours = secs / 3600;

    secs = secs - hours * 3600;

    let minutes = secs / 60;

    secs = secs - minutes * 60;

    return if days > 0 {
        format!("{}d:{:02}:{:02}:{:02}", days, hours, minutes, secs)
    } else {
        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    };
}

#[derive(Debug)]
pub enum ParseDurationError {
    ParseIntError(ParseIntError),
    Other(String),
}

impl From<ParseIntError> for ParseDurationError {
    fn from(src: ParseIntError) -> Self {
        Self::ParseIntError(src)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_minutes() {
        assert_eq!("00:01:00", format_duration_seconds(60));
        assert_eq!("00:01:01", format_duration_seconds(61));

        assert_eq!("00:02:00", format_duration_seconds(60 * 2));
        assert_eq!("00:02:01", format_duration_seconds(60 * 2 + 1));

        assert_eq!("00:59:00", format_duration_seconds(60 * 59));
        assert_eq!("00:59:59", format_duration_seconds(60 * 59 + 59));
    }

    #[test]
    fn test_hours() {
        assert_eq!("01:00:00", format_duration_seconds(60 * 60));
        assert_eq!("01:01:00", format_duration_seconds(60 * 60 + 60));
        assert_eq!("01:01:01", format_duration_seconds(60 * 60 + 61));
    }

    #[test]
    fn test_days() {
        assert_eq!("1d:00:00:00", format_duration_seconds(60 * 60 * 24));
        assert_eq!("1d:00:00:01", format_duration_seconds(60 * 60 * 24 + 1));
    }

    #[test]
    fn test_parse_with_days() {
        let src = "15d 1:01:01";

        let duration = Duration::from_str(src).unwrap();

        assert_eq!("15d:01:01:01", duration_to_string(duration));
    }

    #[test]
    fn test_parse_days() {
        let src = "15d";

        let duration = Duration::from_str(src).unwrap();

        assert_eq!("15d:00:00:00", duration_to_string(duration));
    }
}
