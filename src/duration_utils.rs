use std::{num::ParseIntError, time::Duration};

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
    let hms: Vec<&str> = str.split(":").collect();

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

    let err = ParseDurationError::Other(format!("Can not parse duration: {}", str));
    Err(err)
}

pub fn duration_to_string(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        return format!("{:?}", d);
    }

    return format_duration(secs as i64);
}

const SECS_PER_DAY: i64 = 3600 * 24;

fn format_duration(mut secs: i64) -> String {
    let days = secs / SECS_PER_DAY;

    secs = secs - days * SECS_PER_DAY;

    let hours = secs / 3600;

    secs = secs - hours * 3600;

    let minutes = secs / 60;

    secs = secs - minutes * 60;

    return if days > 0 {
        format!("{}:{:02}:{:02}:{:02}", days, hours, minutes, secs)
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
