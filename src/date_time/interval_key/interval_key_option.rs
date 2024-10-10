use crate::date_time::*;

pub trait IntervalKeyOption {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String>;
    fn to_value(src: DateTimeAsMicroseconds) -> i64;
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct YearKey;

impl IntervalKeyOption for YearKey {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        let date_time_struct = DateTimeStruct {
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
        };

        let result: Option<DateTimeAsMicroseconds> =
            date_time_struct.to_date_time_as_microseconds();

        match result {
            Some(value) => Ok(value),
            None => Err(format!("{} is not a valid year", value)),
        }
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        let date_time_struct: DateTimeStruct = src.into();
        date_time_struct.year as i64
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MonthKey;

impl IntervalKeyOption for MonthKey {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        let year = value / 100;
        let month = value % 100;

        let date_time_struct = DateTimeStruct {
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
        };

        let result: Option<DateTimeAsMicroseconds> =
            date_time_struct.to_date_time_as_microseconds();

        match result {
            Some(value) => Ok(value),
            None => Err(format!("{} is not a valid year+month key", value)),
        }
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        let date_time_struct: DateTimeStruct = src.into();
        date_time_struct.year as i64 * 100 + date_time_struct.month as i64
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DayKey;

impl IntervalKeyOption for DayKey {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        let year = value / 10000;
        let month = (value % 10000) / 100;
        let day = value % 100;

        let date_time_struct = DateTimeStruct {
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
        };

        let result: Option<DateTimeAsMicroseconds> =
            date_time_struct.to_date_time_as_microseconds();

        match result {
            Some(value) => Ok(value),
            None => Err(format!("{} is not a valid year+month+day key", value)),
        }
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        let date_time_struct: DateTimeStruct = src.into();
        date_time_struct.year as i64 * 10000
            + date_time_struct.month as i64 * 100
            + date_time_struct.day as i64
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HourKey;

impl IntervalKeyOption for HourKey {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        let year = value / 1000000;
        let month = (value % 1000000) / 10000;
        let day = (value % 10000) / 100;
        let hour = value % 100;

        let date_time_struct = DateTimeStruct {
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
        };

        let result: Option<DateTimeAsMicroseconds> =
            date_time_struct.to_date_time_as_microseconds();

        match result {
            Some(value) => Ok(value),
            None => Err(format!("{} is not a valid year+month+day+hour key", value)),
        }
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        let date_time_struct: DateTimeStruct = src.into();
        (date_time_struct.year as i64) * 1000000
            + date_time_struct.month as i64 * 10000
            + date_time_struct.day as i64 * 100
            + date_time_struct.time.hour as i64
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MinuteKey;

impl IntervalKeyOption for MinuteKey {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        let year = value / 100000000;
        let month = (value % 100000000) / 1000000;
        let day = (value % 1000000) / 10000;
        let hour = (value % 10000) / 100;
        let min = value % 100;

        let date_time_struct = DateTimeStruct {
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
        };

        let result: Option<DateTimeAsMicroseconds> =
            date_time_struct.to_date_time_as_microseconds();

        match result {
            Some(value) => Ok(value),
            None => Err(format!("{} is not a valid year+month+day+hour key", value)),
        }
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        let date_time_struct: DateTimeStruct = src.into();
        (date_time_struct.year as i64) * 100000000
            + date_time_struct.month as i64 * 1000000
            + date_time_struct.day as i64 * 10000
            + date_time_struct.time.hour as i64 * 100
            + date_time_struct.time.min as i64
    }
}
