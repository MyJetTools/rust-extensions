pub mod year {
    use crate::date_time::*;
    pub fn to_date_time_struct(value: i64) -> DateTimeStruct {
        DateTimeStruct {
            year: value as i32,
            month: 1,
            day: 1,
            time: TimeStruct {
                hour: 0,
                min: 0,
                sec: 0,
                micros: 0,
            },
            dow: None,
        }
    }

    pub fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        let result: Option<DateTimeAsMicroseconds> =
            to_date_time_struct(value).to_date_time_as_microseconds();

        match result {
            Some(value) => Ok(value),
            None => Err(format!("{} is not a valid year", value)),
        }
    }

    pub fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        let date_time_struct: DateTimeStruct = src.into();
        date_time_struct.year as i64
    }
}

pub mod month {
    use crate::date_time::*;

    pub fn to_date_time_struct(value: i64) -> DateTimeStruct {
        let year = value / 100;
        let month = value % 100;

        DateTimeStruct {
            year: year as i32,
            month: month as u32,
            day: 1,
            time: TimeStruct {
                hour: 0,
                min: 0,
                sec: 0,
                micros: 0,
            },
            dow: None,
        }
    }

    pub fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        let result: Option<DateTimeAsMicroseconds> =
            to_date_time_struct(value).to_date_time_as_microseconds();

        match result {
            Some(value) => Ok(value),
            None => Err(format!("{} is not a valid year+month key", value)),
        }
    }

    pub fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        let date_time_struct: DateTimeStruct = src.into();
        date_time_struct.year as i64 * 100 + date_time_struct.month as i64
    }

    #[cfg(test)]
    mod tests {

        #[test]
        fn test_dt_struct() {
            let key = 202502;

            let dt_struct = super::to_date_time_struct(key);

            assert_eq!(dt_struct.year, 2025);
            assert_eq!(dt_struct.month, 02);
            assert_eq!(dt_struct.day, 01);
        }
    }
}

pub mod day {
    use crate::date_time::*;

    pub fn to_date_time_struct(value: i64) -> DateTimeStruct {
        let year = value / 10000;
        let month = (value % 10000) / 100;
        let day = value % 100;

        DateTimeStruct {
            year: year as i32,
            month: month as u32,
            day: day as u32,
            time: TimeStruct {
                hour: 0,
                min: 0,
                sec: 0,
                micros: 0,
            },
            dow: None,
        }
    }

    pub fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        let result: Option<DateTimeAsMicroseconds> =
            to_date_time_struct(value).to_date_time_as_microseconds();

        match result {
            Some(value) => Ok(value),
            None => Err(format!("{} is not a valid year+month+day key", value)),
        }
    }

    pub fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        let date_time_struct: DateTimeStruct = src.into();
        date_time_struct.year as i64 * 10000
            + date_time_struct.month as i64 * 100
            + date_time_struct.day as i64
    }

    #[cfg(test)]
    mod tests {

        #[test]
        fn test_dt_struct() {
            let key = 20250203;

            let dt_struct = super::to_date_time_struct(key);

            assert_eq!(dt_struct.year, 2025);
            assert_eq!(dt_struct.month, 02);
            assert_eq!(dt_struct.day, 03);
        }
    }
}

pub mod hour {
    use crate::date_time::*;

    pub fn to_date_time_struct(value: i64) -> DateTimeStruct {
        let year = value / 1000000;
        let month = (value % 1000000) / 10000;
        let day = (value % 10000) / 100;
        let hour = value % 100;
        DateTimeStruct {
            year: year as i32,
            month: month as u32,
            day: day as u32,
            time: TimeStruct {
                hour: hour as u32,
                min: 0,
                sec: 0,
                micros: 0,
            },
            dow: None,
        }
    }

    pub fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        let result: Option<DateTimeAsMicroseconds> =
            to_date_time_struct(value).to_date_time_as_microseconds();

        match result {
            Some(value) => Ok(value),
            None => Err(format!("{} is not a valid year+month+day+hour key", value)),
        }
    }

    pub fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        let date_time_struct: DateTimeStruct = src.into();
        (date_time_struct.year as i64) * 1000000
            + date_time_struct.month as i64 * 10000
            + date_time_struct.day as i64 * 100
            + date_time_struct.time.hour as i64
    }
}

pub mod hour2 {
    use crate::date_time::*;

    pub fn to_date_time_struct(value: i64) -> DateTimeStruct {
        let year = value / 1000000;
        let month = (value % 1000000) / 10000;
        let day = (value % 10000) / 100;
        // Normalize to the slot start, so a raw value with an off-slot hour
        // round-trips the same way as a key produced by to_value.
        let hour = (value % 100) / 2 * 2;
        DateTimeStruct {
            year: year as i32,
            month: month as u32,
            day: day as u32,
            time: TimeStruct {
                hour: hour as u32,
                min: 0,
                sec: 0,
                micros: 0,
            },
            dow: None,
        }
    }

    pub fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        let result: Option<DateTimeAsMicroseconds> =
            to_date_time_struct(value).to_date_time_as_microseconds();

        match result {
            Some(value) => Ok(value),
            None => Err(format!("{} is not a valid 2-hour interval key", value)),
        }
    }

    pub fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        let date_time_struct: DateTimeStruct = src.into();
        let hour_slot = date_time_struct.time.hour as i64 / 2; // Convert hours to 2-hour slot (0-11)

        (date_time_struct.year as i64) * 1000000
            + date_time_struct.month as i64 * 10000
            + date_time_struct.day as i64 * 100
            + hour_slot * 2
    }
}

pub mod hour4 {
    use crate::date_time::*;

    pub fn to_date_time_struct(value: i64) -> DateTimeStruct {
        let year = value / 1000000;
        let month = (value % 1000000) / 10000;
        let day = (value % 10000) / 100;
        // Normalize to the slot start, so a raw value with an off-slot hour
        // round-trips the same way as a key produced by to_value.
        let hour = (value % 100) / 4 * 4;
        DateTimeStruct {
            year: year as i32,
            month: month as u32,
            day: day as u32,
            time: TimeStruct {
                hour: hour as u32,
                min: 0,
                sec: 0,
                micros: 0,
            },
            dow: None,
        }
    }

    pub fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        let result: Option<DateTimeAsMicroseconds> =
            to_date_time_struct(value).to_date_time_as_microseconds();

        match result {
            Some(value) => Ok(value),
            None => Err(format!("{} is not a valid 4-hour interval key", value)),
        }
    }

    pub fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        let date_time_struct: DateTimeStruct = src.into();
        let hour_slot = date_time_struct.time.hour as i64 / 4; // Convert hours to 4-hour slot (0-5)

        (date_time_struct.year as i64) * 1000000
            + date_time_struct.month as i64 * 10000
            + date_time_struct.day as i64 * 100
            + hour_slot * 4
    }
}

pub mod minute {
    use crate::date_time::*;

    pub fn to_date_time_struct(value: i64) -> DateTimeStruct {
        let year = value / 100000000;
        let month = (value % 100000000) / 1000000;
        let day = (value % 1000000) / 10000;
        let hour = (value % 10000) / 100;
        let min = value % 100;

        DateTimeStruct {
            year: year as i32,
            month: month as u32,
            day: day as u32,
            time: TimeStruct {
                hour: hour as u32,
                min: min as u32,
                sec: 0,
                micros: 0,
            },
            dow: None,
        }
    }

    pub fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        let result: Option<DateTimeAsMicroseconds> =
            to_date_time_struct(value).to_date_time_as_microseconds();

        match result {
            Some(value) => Ok(value),
            None => Err(format!(
                "{} is not a valid year+month+day+hour+minute key",
                value
            )),
        }
    }

    pub fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        let date_time_struct: DateTimeStruct = src.into();
        (date_time_struct.year as i64) * 100000000
            + date_time_struct.month as i64 * 1000000
            + date_time_struct.day as i64 * 10000
            + date_time_struct.time.hour as i64 * 100
            + date_time_struct.time.min as i64
    }
}

pub mod min5 {
    use crate::date_time::*;

    pub fn to_date_time_struct(value: i64) -> DateTimeStruct {
        let year = value / 100000000;
        let month = (value % 100000000) / 1000000;
        let day = (value % 1000000) / 10000;
        let hour = (value % 10000) / 100;
        // Normalize to the slot start, so a raw value with an off-slot minute
        // round-trips the same way as a key produced by to_value.
        let min = (value % 100) / 5 * 5;

        DateTimeStruct {
            year: year as i32,
            month: month as u32,
            day: day as u32,
            time: TimeStruct {
                hour: hour as u32,
                min: min as u32,
                sec: 0,
                micros: 0,
            },
            dow: None,
        }
    }

    pub fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        let result: Option<DateTimeAsMicroseconds> =
            to_date_time_struct(value).to_date_time_as_microseconds();

        match result {
            Some(value) => Ok(value),
            None => Err(format!("{} is not a valid 5-minute interval key", value)),
        }
    }

    pub fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        let date_time_struct: DateTimeStruct = src.into();
        let minute_slot = date_time_struct.time.min as i64 / 5; // Convert minutes to 5-min slot (0-11)

        (date_time_struct.year as i64) * 100000000
            + date_time_struct.month as i64 * 1000000
            + date_time_struct.day as i64 * 10000
            + date_time_struct.time.hour as i64 * 100
            + minute_slot * 5
    }
}

pub mod min15 {
    use crate::date_time::*;

    pub fn to_date_time_struct(value: i64) -> DateTimeStruct {
        let year = value / 100000000;
        let month = (value % 100000000) / 1000000;
        let day = (value % 1000000) / 10000;
        let hour = (value % 10000) / 100;
        // Normalize to the slot start, so a raw value with an off-slot minute
        // round-trips the same way as a key produced by to_value.
        let min = (value % 100) / 15 * 15;

        DateTimeStruct {
            year: year as i32,
            month: month as u32,
            day: day as u32,
            time: TimeStruct {
                hour: hour as u32,
                min: min as u32,
                sec: 0,
                micros: 0,
            },
            dow: None,
        }
    }

    pub fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        let result: Option<DateTimeAsMicroseconds> =
            to_date_time_struct(value).to_date_time_as_microseconds();

        match result {
            Some(value) => Ok(value),
            None => Err(format!("{} is not a valid 15-minute interval key", value)),
        }
    }

    pub fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        let date_time_struct: DateTimeStruct = src.into();
        let minute_slot = date_time_struct.time.min as i64 / 15; // Convert minutes to 15-min slot (0-3)

        (date_time_struct.year as i64) * 100000000
            + date_time_struct.month as i64 * 1000000
            + date_time_struct.day as i64 * 10000
            + date_time_struct.time.hour as i64 * 100
            + minute_slot * 15
    }
}

pub mod min30 {
    use crate::date_time::*;

    pub fn to_date_time_struct(value: i64) -> DateTimeStruct {
        let year = value / 100000000;
        let month = (value % 100000000) / 1000000;
        let day = (value % 1000000) / 10000;
        let hour = (value % 10000) / 100;
        // Normalize to the slot start, so a raw value with an off-slot minute
        // round-trips the same way as a key produced by to_value.
        let min = (value % 100) / 30 * 30;

        DateTimeStruct {
            year: year as i32,
            month: month as u32,
            day: day as u32,
            time: TimeStruct {
                hour: hour as u32,
                min: min as u32,
                sec: 0,
                micros: 0,
            },
            dow: None,
        }
    }

    pub fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        let result: Option<DateTimeAsMicroseconds> =
            to_date_time_struct(value).to_date_time_as_microseconds();

        match result {
            Some(value) => Ok(value),
            None => Err(format!("{} is not a valid 30-minute interval key", value)),
        }
    }

    pub fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        let date_time_struct: DateTimeStruct = src.into();
        let minute_slot = date_time_struct.time.min as i64 / 30; // Convert minutes to 30-min slot (0-1)

        (date_time_struct.year as i64) * 100000000
            + date_time_struct.month as i64 * 1000000
            + date_time_struct.day as i64 * 10000
            + date_time_struct.time.hour as i64 * 100
            + minute_slot * 30
    }
}

// A week is not a single calendar field, so it is encoded as the YYYYMMDD date
// of the week start (reusing the `day` layout): numeric order stays
// chronological and the key is a real, decodable date.
pub mod week_monday {
    use crate::date_time::*;
    use std::time::Duration;

    const DAY: u64 = 24 * 60 * 60;

    /// Cuts a timestamp back to 00:00 of the Monday that starts its week.
    pub fn to_week_start(src: DateTimeAsMicroseconds) -> DateTimeAsMicroseconds {
        let date_time_struct: DateTimeStruct = (&src).into();
        let days_back = date_time_struct.get_day_of_week().num_days_from_monday() as u64;
        let week_start = src.sub(Duration::from_secs(days_back * DAY));
        // Drop the time-of-day so only the week-start date survives.
        super::day::to_date_time(super::day::to_value(week_start)).unwrap()
    }

    pub fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        // Normalize to the week start, so a raw value that is not a Monday
        // round-trips the same way as a key produced by to_value.
        let dt = super::day::to_date_time(value)?;
        Ok(to_week_start(dt))
    }

    pub fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::day::to_value(to_week_start(src))
    }
}

pub mod week_sunday {
    use crate::date_time::*;
    use std::time::Duration;

    const DAY: u64 = 24 * 60 * 60;

    /// Cuts a timestamp back to 00:00 of the Sunday that starts its week.
    pub fn to_week_start(src: DateTimeAsMicroseconds) -> DateTimeAsMicroseconds {
        let date_time_struct: DateTimeStruct = (&src).into();
        let days_back = date_time_struct.get_day_of_week().num_days_from_sunday() as u64;
        let week_start = src.sub(Duration::from_secs(days_back * DAY));
        // Drop the time-of-day so only the week-start date survives.
        super::day::to_date_time(super::day::to_value(week_start)).unwrap()
    }

    pub fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        // Normalize to the week start, so a raw value that is not a Sunday
        // round-trips the same way as a key produced by to_value.
        let dt = super::day::to_date_time(value)?;
        Ok(to_week_start(dt))
    }

    pub fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::day::to_value(to_week_start(src))
    }
}
