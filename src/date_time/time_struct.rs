use chrono::Weekday;

#[derive(Default, Debug, Clone)]
pub struct TimeStruct {
    pub hour: u32,
    pub min: u32,
    pub sec: u32,
    pub micros: u32,
}

impl TimeStruct {
    pub fn to_micro_seconds(&self) -> u64 {
        self.micros as u64
            + self.sec as u64 * 1_000_000
            + self.min as u64 * 60 * 1_000_000
            + self.hour as u64 * 60 * 60 * 1_000_000
    }

    pub fn to_micro_second_withing_week(&self, week_day: Weekday) -> u64 {
        let wd = week_day as u64 * 24 * 60 * 60 * 1_000_000;
        let micros = self.to_micro_seconds();
        micros + wd
    }

    pub fn is_grater_then(&self, other: &Self) -> bool {
        self.to_micro_seconds() > other.to_micro_seconds()
    }

    pub fn is_grater_or_eq_then(&self, other: &Self) -> bool {
        self.to_micro_seconds() >= other.to_micro_seconds()
    }

    pub fn is_equal_to(&self, other: &Self) -> bool {
        self.to_micro_seconds() == other.to_micro_seconds()
    }

    pub fn is_less_then(&self, other: &Self) -> bool {
        self.to_micro_seconds() < other.to_micro_seconds()
    }

    pub fn is_less_or_eq_then(&self, other: &Self) -> bool {
        self.to_micro_seconds() <= other.to_micro_seconds()
    }

    pub fn push_time_no_micros_to_str(&self, dest: &mut String) {
        if self.hour < 10 {
            dest.push('0');
        }

        dest.push_str(self.hour.to_string().as_str());

        dest.push(':');

        if self.min < 10 {
            dest.push('0');
        }

        dest.push_str(self.min.to_string().as_str());

        dest.push(':');

        if self.sec < 10 {
            dest.push('0');
        }

        dest.push_str(self.sec.to_string().as_str());
    }
    pub fn parse_from_str(src: &str) -> Option<Self> {
        let mut v = Vec::with_capacity(2);

        let mut hour = 0;
        let mut min = 0;
        let mut sec = 0;

        let mut micros = 0;

        let mut no = 0;

        let src = src.as_bytes();

        for i in 0..src.len() {
            let b = src[i];
            if b == b':' {
                match no {
                    0 => {
                        no += 1;
                        hour = match super::utils::parse_two_digits(v.as_slice()) {
                            Some(result) => result,
                            None => return None,
                        }
                    }
                    1 => {
                        no += 1;
                        min = match super::utils::parse_two_digits(v.as_slice()) {
                            Some(result) => result,
                            None => return None,
                        }
                    }

                    _ => return None,
                }

                v.clear();
                continue;
            }
            if b == b'.' {
                sec = match super::utils::parse_two_digits(v.as_slice()) {
                    Some(result) => result,
                    None => return None,
                };
                v.clear();
                micros = parse_microseconds(&src[i + 1..]);
                break;
            } else {
                v.push(b)
            }
        }

        if v.len() > 0 {
            match no {
                0 => {
                    hour = match super::utils::parse_two_digits(v.as_slice()) {
                        Some(result) => result,
                        None => return None,
                    }
                }
                1 => {
                    min = match super::utils::parse_two_digits(v.as_slice()) {
                        Some(result) => result,
                        None => return None,
                    }
                }
                2 => {
                    sec = match super::utils::parse_two_digits(v.as_slice()) {
                        Some(result) => result,
                        None => return None,
                    }
                }
                _ => return None,
            }
        }

        Self {
            hour,
            min,
            sec,
            micros,
        }
        .into()
    }

    pub fn parse_rfc_3339_time(src: &[u8]) -> Option<Self> {
        let hour = super::utils::parse_two_digits(&src[0..2])?;

        let min = super::utils::parse_two_digits(&src[3..5])?;

        let sec = super::utils::parse_two_digits(&src[6..8])?;

        let mut micros = 0;

        if src.len() > 9 {
            let d = &src[9..];
            micros = parse_microseconds(d)
        }

        Self {
            hour,
            min,
            sec,
            micros,
        }
        .into()
    }

    pub fn parse_rfc_3339_url_encoded_time(src: &[u8]) -> Option<Self> {
        let hour = super::utils::parse_two_digits(&src[0..2])?;

        let min = super::utils::parse_two_digits(&src[5..7])?;

        let sec = super::utils::parse_two_digits(&src[10..12])?;

        let mut micros = 0;

        if src.len() > 12 {
            let d = &src[13..];
            println!("ms:{}", std::str::from_utf8(d).unwrap());
            micros = parse_microseconds(d)
        }

        Self {
            hour,
            min,
            sec,
            micros,
        }
        .into()
    }
}

#[inline]
pub fn parse_microseconds(src: &[u8]) -> u32 {
    let mut multiplier: u32 = 100000;

    let mut result: u32 = 0;

    for b in src {
        let b = super::utils::parse_number(*b);

        if b.is_none() {
            return result;
        }

        result += b.unwrap() * multiplier;

        if multiplier == 1 {
            break;
        }

        multiplier /= 10;
    }

    result
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    pub fn parse_time_basic_from_str() {
        let time = TimeStruct::parse_from_str("12:05:09").unwrap();

        assert_eq!(time.hour, 12);
        assert_eq!(time.min, 5);
        assert_eq!(time.sec, 9);
        assert_eq!(time.micros, 0);
    }

    #[test]
    pub fn parse_time_basic_from_str_micros_1() {
        let time = TimeStruct::parse_from_str("12:05:09.1").unwrap();

        assert_eq!(time.hour, 12);
        assert_eq!(time.min, 5);
        assert_eq!(time.sec, 9);
        assert_eq!(time.micros, 100000);
    }

    #[test]
    pub fn parse_time_basic_from_str_micros_12() {
        let time = TimeStruct::parse_from_str("12:05:09.12").unwrap();

        assert_eq!(time.hour, 12);
        assert_eq!(time.min, 5);
        assert_eq!(time.sec, 9);
        assert_eq!(time.micros, 120000);
    }

    #[test]
    pub fn parse_time_basic_from_str_micros_123() {
        let time = TimeStruct::parse_from_str("12:05:09.123").unwrap();

        assert_eq!(time.hour, 12);
        assert_eq!(time.min, 5);
        assert_eq!(time.sec, 9);
        assert_eq!(time.micros, 123000);
    }

    #[test]
    pub fn parse_time_basic_from_str_micros_123_z() {
        let time = TimeStruct::parse_from_str("12:05:09.123Z").unwrap();

        assert_eq!(time.hour, 12);
        assert_eq!(time.min, 5);
        assert_eq!(time.sec, 9);
        assert_eq!(time.micros, 123000);
    }

    #[test]
    pub fn parse_time_basic() {
        let time = TimeStruct::parse_rfc_3339_time("12:05:09".as_bytes()).unwrap();

        assert_eq!(time.hour, 12);
        assert_eq!(time.min, 5);
        assert_eq!(time.sec, 9);
        assert_eq!(time.micros, 0);
    }

    #[test]
    pub fn parse_time_basic_micros_1() {
        let time = TimeStruct::parse_rfc_3339_time("12:05:09.1".as_bytes()).unwrap();

        assert_eq!(time.hour, 12);
        assert_eq!(time.min, 5);
        assert_eq!(time.sec, 9);
        assert_eq!(time.micros, 100000);
    }

    #[test]
    pub fn parse_time_basic_micros_12() {
        let time = TimeStruct::parse_rfc_3339_time("12:05:09.12".as_bytes()).unwrap();

        assert_eq!(time.hour, 12);
        assert_eq!(time.min, 5);
        assert_eq!(time.sec, 9);
        assert_eq!(time.micros, 120000);
    }

    #[test]
    pub fn parse_time_basic_micros_123() {
        let time = TimeStruct::parse_rfc_3339_time("12:05:09.123".as_bytes()).unwrap();

        assert_eq!(time.hour, 12);
        assert_eq!(time.min, 5);
        assert_eq!(time.sec, 9);
        assert_eq!(time.micros, 123000);
    }

    #[test]
    pub fn parse_time_basic_micros_1234() {
        let time = TimeStruct::parse_rfc_3339_time("12:05:09.1234".as_bytes()).unwrap();

        assert_eq!(time.hour, 12);
        assert_eq!(time.min, 5);
        assert_eq!(time.sec, 9);
        assert_eq!(time.micros, 123400);
    }

    #[test]
    pub fn parse_time_basic_micros_12345() {
        let time = TimeStruct::parse_rfc_3339_time("12:05:09.12345".as_bytes()).unwrap();

        assert_eq!(time.hour, 12);
        assert_eq!(time.min, 5);
        assert_eq!(time.sec, 9);
        assert_eq!(time.micros, 123450);
    }

    #[test]
    pub fn parse_time_basic_micros_123456() {
        let time = TimeStruct::parse_rfc_3339_time("12:05:09.123456".as_bytes()).unwrap();

        assert_eq!(time.hour, 12);
        assert_eq!(time.min, 5);
        assert_eq!(time.sec, 9);
        assert_eq!(time.micros, 123456);
    }

    #[test]
    pub fn parse_time_basic_micros_123456_z() {
        let time = TimeStruct::parse_rfc_3339_time("12:05:09.123456Z".as_bytes()).unwrap();

        assert_eq!(time.hour, 12);
        assert_eq!(time.min, 5);
        assert_eq!(time.sec, 9);
        assert_eq!(time.micros, 123456);
    }

    #[test]
    pub fn parse_time_url_encoded() {
        let time =
            TimeStruct::parse_rfc_3339_url_encoded_time("12%3A05%3A09.123456Z".as_bytes()).unwrap();

        assert_eq!(time.hour, 12);
        assert_eq!(time.min, 5);
        assert_eq!(time.sec, 9);
        assert_eq!(time.micros, 123456);
    }

    #[test]
    pub fn test_parse_microseconds() {
        let src = "605Z";
        let result = parse_microseconds(src.as_bytes());

        assert_eq!(605000, result);
    }

    #[test]
    pub fn test_parse_microseconds_full_case() {
        let src = "605243";
        let result = parse_microseconds(src.as_bytes());

        assert_eq!(605243, result);
    }
    #[test]
    pub fn test_greater() {
        let time1 = TimeStruct {
            hour: 12,
            min: 5,
            sec: 9,
            micros: 123456,
        };

        let time2 = TimeStruct {
            hour: 12,
            min: 5,
            sec: 9,
            micros: 123455,
        };

        println!(
            "time1:{} time2:{}",
            time1.to_micro_seconds(),
            time2.to_micro_seconds()
        );

        assert!(time1.is_grater_then(&time2));
    }

    #[test]
    pub fn test_greater_next_day() {
        let time1 = TimeStruct {
            hour: 0,
            min: 0,
            sec: 0,
            micros: 0,
        };

        let time2 = TimeStruct {
            hour: 23,
            min: 59,
            sec: 59,
            micros: 999999,
        };

        let ms1 = time1.to_micro_second_withing_week(Weekday::Tue);
        let ms2 = time2.to_micro_second_withing_week(Weekday::Mon);

        println!("time1:{} time2:{}", ms1, ms2);

        assert!(ms2 < ms1);
    }
}
