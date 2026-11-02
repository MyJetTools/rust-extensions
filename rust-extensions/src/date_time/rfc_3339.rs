use super::{DateTimeAsMicroseconds, DateTimeStruct, TimeStruct};

impl DateTimeStruct {
    /// RFC 3339 with the zero offset spelled `+00:00`. The fraction takes the fewest digits out
    /// of 0, 3 and 6 that lose nothing.
    pub fn to_rfc3339(&self) -> String {
        let mut result = String::with_capacity(32);

        self.push_rfc3339_no_fraction_to_str(&mut result);

        if self.time.micros % 1000 != 0 {
            self.time.push_micros_to_str(&mut result);
        } else if self.time.micros != 0 {
            result.push('.');
            super::utils::push_zero_padded(&mut result, self.time.micros / 1000, 3);
        }

        result.push_str("+00:00");

        result
    }

    /// RFC 3339 with a `Z` suffix and all six fraction digits.
    pub fn to_rfc3339_utc(&self) -> String {
        let mut result = String::with_capacity(32);

        self.push_rfc3339_no_fraction_to_str(&mut result);
        self.time.push_micros_to_str(&mut result);
        result.push('Z');

        result
    }

    /// `2021-04-25T17:30:03`. A year outside of four digits gets a sign, as ISO 8601 asks.
    pub(super) fn push_rfc3339_no_fraction_to_str(&self, dest: &mut String) {
        if self.year >= 0 && self.year <= 9999 {
            super::utils::push_zero_padded(dest, self.year as u32, 4);
        } else {
            dest.push_str(format!("{:+05}", self.year).as_str());
        }

        dest.push('-');
        super::utils::push_zero_padded(dest, self.month, 2);
        dest.push('-');
        super::utils::push_zero_padded(dest, self.day, 2);
        dest.push('T');
        self.time.push_time_no_micros_to_str(dest);
    }

    /// The strict RFC 3339, for a value whose offset matters. Unlike [`Self::parse_rfc3339_str`],
    /// which takes shorter forms and ignores the offset, this one checks the whole string and
    /// requires the offset. Returns the local date-time and the offset in minutes.
    ///
    /// Takes what the RFC leaves open: `t` and a space for `T`, `z` for `Z`, any number of fraction
    /// digits (the ones past microseconds are dropped), and a leap second `:60` on any minute.
    pub(super) fn parse_rfc3339_with_offset(src: &str) -> Option<(Self, i32)> {
        let as_bytes = src.as_bytes();

        if as_bytes.len() < 19
            || as_bytes[4] != b'-'
            || as_bytes[7] != b'-'
            || !matches!(as_bytes[10], b'T' | b't' | b' ')
            || as_bytes[13] != b':'
            || as_bytes[16] != b':'
        {
            return None;
        }

        let year = super::utils::parse_four_digits(&as_bytes[0..4])?;
        let month = super::utils::parse_two_digits(&as_bytes[5..7])?;
        let day = super::utils::parse_two_digits(&as_bytes[8..10])?;
        let hour = super::utils::parse_two_digits(&as_bytes[11..13])?;
        let min = super::utils::parse_two_digits(&as_bytes[14..16])?;
        let mut sec = super::utils::parse_two_digits(&as_bytes[17..19])?;

        let mut micros = 0;
        let mut offset_position = 19;

        if as_bytes.get(19) == Some(&b'.') {
            let fraction = &as_bytes[20..];
            let digits = fraction.iter().take_while(|b| b.is_ascii_digit()).count();

            if digits == 0 {
                return None;
            }

            micros = super::parse_microseconds(fraction);
            offset_position = 20 + digits;
        }

        if sec == 60 {
            sec = 59;
            micros += 1_000_000;
        }

        let offset_in_minutes = parse_rfc3339_offset(&src[offset_position..])?;

        let date_time = Self {
            year,
            month,
            day,
            time: TimeStruct {
                hour,
                min,
                sec,
                micros,
            },
            dow: None,
        };

        Some((date_time, offset_in_minutes))
    }

    pub fn parse_rfc3339_str(src: &[u8]) -> Option<Self> {
        if src.len() < 10 {
            return None;
        }

        let year = super::utils::parse_four_digits(&src[0..4])? as i32;

        let month = super::utils::parse_two_digits(&src[5..7])?;

        let day = super::utils::parse_two_digits(&src[8..10])?;

        let time = if src.len() > 10 {
            TimeStruct::parse_rfc_3339_time(&src[11..])?
        } else {
            TimeStruct::default()
        };

        return Some(Self {
            year,
            month,
            day,
            time,
            dow: None,
        });
    }

    pub fn parse_rfc3339_url_encoded_str(src: &[u8]) -> Option<Self> {
        if src.len() < 10 {
            return None;
        }

        let year = super::utils::parse_four_digits(&src[0..4])?;

        let month = super::utils::parse_two_digits(&src[5..7])?;

        let day = super::utils::parse_two_digits(&src[8..10])?;

        let time = if src.len() > 10 {
            TimeStruct::parse_rfc_3339_url_encoded_time(&src[11..])?
        } else {
            TimeStruct::default()
        };

        return Some(Self {
            year,
            month,
            day,
            time,
            dow: None,
        });
    }

    pub fn to_unix_microseconds(&self) -> Option<i64> {
        if !super::calendar::is_valid_date(self.year, self.month, self.day) || !self.time.is_valid()
        {
            return None;
        }

        let days = super::calendar::days_from_civil(self.year, self.month, self.day);

        Some(days * super::MICRO_SECONDS_IN_ONE_DAY + self.time.to_micro_seconds() as i64)
    }

    pub fn to_date_time_as_microseconds(&self) -> Option<DateTimeAsMicroseconds> {
        let unix_microseconds = self.to_unix_microseconds()?;
        Some(DateTimeAsMicroseconds::new(unix_microseconds))
    }
}

/// `Z`, `+01:00` or `-05:00` as minutes east of UTC. The offset stays under 24 hours, and the
/// `−` (U+2212) reads as a `-`.
fn parse_rfc3339_offset(src: &str) -> Option<i32> {
    if src == "Z" || src == "z" {
        return Some(0);
    }

    let (is_negative, digits) = if let Some(digits) = src.strip_prefix('+') {
        (false, digits)
    } else if let Some(digits) = src.strip_prefix('-') {
        (true, digits)
    } else {
        (true, src.strip_prefix('\u{2212}')?)
    };

    let digits = digits.as_bytes();

    if digits.len() != 5 || digits[2] != b':' {
        return None;
    }

    let hours = super::utils::parse_two_digits(&digits[0..2])?;
    let minutes = super::utils::parse_two_digits(&digits[3..5])?;

    if hours > 23 || minutes > 59 {
        return None;
    }

    let result = (hours * 60 + minutes) as i32;

    if is_negative {
        Some(-result)
    } else {
        Some(result)
    }
}

impl<'s> TryInto<DateTimeAsMicroseconds> for &'s DateTimeStruct {
    type Error = ();

    fn try_into(self) -> Result<DateTimeAsMicroseconds, Self::Error> {
        match self.to_date_time_as_microseconds() {
            Some(result) => Ok(result),
            None => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::date_time::DateTimeAsMicroseconds;

    use super::*;

    #[test]
    pub fn test_parse_iso_string() {
        let src = "2021-04-25T17:30:43.605Z";
        let dt = DateTimeStruct::parse_rfc3339_str(src.as_bytes()).unwrap();

        assert_eq!(2021, dt.year);
        assert_eq!(4, dt.month);
        assert_eq!(25, dt.day);

        assert_eq!(17, dt.time.hour);
        assert_eq!(30, dt.time.min);
        assert_eq!(43, dt.time.sec);

        assert_eq!(605000, dt.time.micros);

        let result: DateTimeAsMicroseconds = dt.try_into().unwrap();

        assert_eq!(1619371843605000, result.unix_microseconds);
    }

    #[test]
    pub fn test_parse_url_encoded_iso_string() {
        let src = "2021-04-25T17%3A30%3A43.605Z";
        let dt = DateTimeStruct::parse_rfc3339_url_encoded_str(src.as_bytes()).unwrap();

        let result: DateTimeAsMicroseconds = dt.try_into().unwrap();

        assert_eq!(1619371843605000, result.unix_microseconds);
    }

    #[test]
    pub fn test_parse_iso_string_and_back() {
        let src = "2021-04-25T17:30:43.602432";
        let dt = DateTimeStruct::parse_rfc3339_str(src.as_bytes()).unwrap();

        assert_eq!(2021, dt.year);
        assert_eq!(4, dt.month);
        assert_eq!(25, dt.day);

        assert_eq!(17, dt.time.hour);
        assert_eq!(30, dt.time.min);
        assert_eq!(43, dt.time.sec);

        assert_eq!(602432, dt.time.micros);

        let result: DateTimeAsMicroseconds = dt.try_into().unwrap();

        let dest = result.to_rfc3339();

        assert_eq!(src, &dest[0..26]);
    }

    #[test]
    pub fn test_parse_url_encoded_iso_string_and_back() {
        let src = "2021-04-25T17%3A30%3A43.602432";
        let dt = DateTimeStruct::parse_rfc3339_url_encoded_str(src.as_bytes()).unwrap();

        let dt: DateTimeAsMicroseconds = dt.try_into().unwrap();

        let dest = dt.to_rfc3339();

        assert_eq!("2021-04-25T17:30:43.602432", &dest[0..26]);
    }

    /// Both parsers index fixed offsets, so a short buffer used to panic.
    #[test]
    pub fn test_parse_rfc3339_str_too_short_returns_none() {
        for src in ["", "2", "20", "202", "2021", "2021-", "2021-0", "2021-04-2"] {
            assert!(
                DateTimeStruct::parse_rfc3339_str(src.as_bytes()).is_none(),
                "expected None for {}",
                src
            );
        }
    }

    #[test]
    pub fn test_parse_rfc3339_url_encoded_str_too_short_returns_none() {
        for src in ["", "2021", "2021-04-2"] {
            assert!(
                DateTimeStruct::parse_rfc3339_url_encoded_str(src.as_bytes()).is_none(),
                "expected None for {}",
                src
            );
        }
    }

    /// `parse_iso_string` reaches the time parser directly, so a value truncated
    /// inside the time part must come back as None rather than panic.
    #[test]
    pub fn test_parse_iso_string_truncated_time_returns_none() {
        for src in [
            "2021-04-25T1",
            "2021-04-25T17",
            "2021-04-25T17:",
            "2021-04-25T17:3",
            "2021-04-25T17:30:",
            "2021-04-25T17:30:4",
        ] {
            let _ = DateTimeAsMicroseconds::parse_iso_string(src);
        }
    }

    /// Zero offset is what my-http-utils' client writes via `to_rfc3339()`.
    #[test]
    pub fn test_parse_iso_string_with_numeric_zero_offset() {
        let dt = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:43+00:00").unwrap();
        assert_eq!("2021-04-25T17:30:43.000000Z", dt.to_rfc3339_utc());

        let dt =
            DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:43.605123+00:00").unwrap();
        assert_eq!("2021-04-25T17:30:43.605123Z", dt.to_rfc3339_utc());
    }

    #[test]
    pub fn test_to_rfc3339_takes_as_many_fraction_digits_as_it_needs() {
        for (unix_microseconds, expected) in [
            (1619371843000000, "2021-04-25T17:30:43+00:00"),
            (1619371843605000, "2021-04-25T17:30:43.605+00:00"),
            (1619371843600000, "2021-04-25T17:30:43.600+00:00"),
            (1619371843605123, "2021-04-25T17:30:43.605123+00:00"),
            (1619371843000001, "2021-04-25T17:30:43.000001+00:00"),
        ] {
            assert_eq!(expected, DateTimeAsMicroseconds::new(unix_microseconds).to_rfc3339());
        }
    }

    #[test]
    pub fn test_to_rfc3339_years_outside_of_four_digits_get_a_sign() {
        let dt: DateTimeStruct = DateTimeAsMicroseconds::create(10000, 1, 2, 3, 4, 5, 0).into();
        assert_eq!("+10000-01-02T03:04:05.000000Z", dt.to_rfc3339_utc());

        let dt: DateTimeStruct = DateTimeAsMicroseconds::create(-1, 1, 2, 3, 4, 5, 0).into();
        assert_eq!("-0001-01-02T03:04:05.000000Z", dt.to_rfc3339_utc());
    }

    #[test]
    pub fn test_parse_rfc3339_offset() {
        for (src, expected) in [
            ("Z", Some(0)),
            ("z", Some(0)),
            ("+00:00", Some(0)),
            ("+01:00", Some(60)),
            ("-05:00", Some(-300)),
            ("+05:45", Some(345)),
            ("\u{2212}05:00", Some(-300)),
            ("+23:59", Some(1439)),
            ("+24:00", None),
            ("+01:60", None),
            ("+0100", None),
            ("+01", None),
            ("+01:00 ", None),
            ("Zz", None),
            ("", None),
        ] {
            assert_eq!(expected, super::parse_rfc3339_offset(src), "{:?}", src);
        }
    }
}
