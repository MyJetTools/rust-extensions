use chrono::{Timelike, Weekday};

use super::{DateTimeAsMicroseconds, TimeStruct};

pub struct DateTimeStruct {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub time: TimeStruct,
    pub dow: Option<Weekday>,
}

impl DateTimeStruct {
    pub fn from_str(src: &str) -> Option<Self> {
        let as_bytes = src.as_bytes();

        if as_bytes.len() == 10 && as_bytes[4] == b'-' && as_bytes[7] == b'-' {
            return DateTimeStruct::parse_rfc3339_str(as_bytes);
        }

        if as_bytes.len() == 14 {
            if src >= "19700101000000" && src <= "21501231235959" {
                return DateTimeStruct::parse_compact_date_time(as_bytes);
            }
        }

        if as_bytes[4] == b'-' && as_bytes.len() >= 19 {
            if as_bytes[13] == b'%' {
                return DateTimeStruct::parse_rfc3339_url_encoded_str(as_bytes);
            } else {
                return DateTimeStruct::parse_rfc3339_str(as_bytes);
            }
        }

        return DateTimeStruct::parse_rfc_5322(src);
    }

    pub fn get_day_of_week_as_str(&self) -> &str {
        let dow = match self.dow {
            Some(dow) => dow,
            None => {
                use chrono::Datelike;
                let dt = self.to_date_time_as_microseconds().unwrap();
                let date = dt.to_chrono_utc();
                date.weekday()
            }
        };

        match dow {
            Weekday::Mon => "Mon",
            Weekday::Tue => "Tue",
            Weekday::Wed => "Wed",
            Weekday::Thu => "Thu",
            Weekday::Fri => "Fri",
            Weekday::Sat => "Sat",
            Weekday::Sun => "Sun",
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_any_parse_from_rfc_5322() {
        let result = DateTimeStruct::from_str("Sep  8 18:41:54 2032 GMT").unwrap();

        assert_eq!(2032, result.year);
        assert_eq!(9, result.month);
        assert_eq!(8, result.day);

        assert_eq!(18, result.time.hour);
        assert_eq!(41, result.time.min);
        assert_eq!(54, result.time.sec);

        assert_eq!(0, result.time.micros);
    }
}

impl Into<DateTimeStruct> for DateTimeAsMicroseconds {
    fn into(self) -> DateTimeStruct {
        use chrono::Datelike;
        let dt = self.to_chrono_utc();

        let date = dt.date_naive();

        let time = dt.time();
        DateTimeStruct {
            year: date.year(),
            month: date.month(),
            day: date.day(),
            dow: Some(date.weekday()),
            time: TimeStruct {
                hour: time.hour(),
                min: time.minute(),
                sec: time.second(),
                micros: time.nanosecond() / 1000,
            },
        }
    }
}

impl<'s> Into<DateTimeStruct> for &'s DateTimeAsMicroseconds {
    fn into(self) -> DateTimeStruct {
        use chrono::Datelike;
        let dt = self.to_chrono_utc();

        let date = dt.date_naive();

        let time = dt.time();
        DateTimeStruct {
            year: date.year(),
            month: date.month(),
            day: date.day(),
            dow: Some(date.weekday()),
            time: TimeStruct {
                hour: time.hour(),
                min: time.minute(),
                sec: time.second(),
                micros: time.nanosecond() / 1000,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::date_time::TimeStruct;

    use super::DateTimeStruct;

    #[test]
    fn test() {
        let src = DateTimeStruct {
            year: 2015,
            month: 12,
            day: 23,
            dow: None,
            time: TimeStruct {
                hour: 13,
                min: 11,
                sec: 45,
                micros: 123456,
            },
        };

        let dt = src.to_date_time_as_microseconds().unwrap();

        let dest: DateTimeStruct = dt.into();

        assert_eq!(src.year, dest.year);
        assert_eq!(src.month, dest.month);

        assert_eq!(src.day, dest.day);
        assert_eq!(src.time.hour, dest.time.hour);

        assert_eq!(src.time.min, dest.time.min);

        assert_eq!(src.time.sec, dest.time.sec);

        assert_eq!(src.time.micros, dest.time.micros);
    }
}
