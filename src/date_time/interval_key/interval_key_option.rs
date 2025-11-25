use crate::date_time::*;

pub trait IntervalKeyOption {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String>;
    fn to_value(src: DateTimeAsMicroseconds) -> i64;
    fn to_dt_interval(value: i64) -> DateTimeInterval;
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct YearKey;

impl IntervalKeyOption for YearKey {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        super::utils::year::to_date_time(value)
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::utils::year::to_value(src)
    }

    fn to_dt_interval(value: i64) -> DateTimeInterval {
        DateTimeInterval::Year(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct MonthKey;

impl IntervalKeyOption for MonthKey {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        super::utils::month::to_date_time(value)
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::utils::month::to_value(src)
    }

    fn to_dt_interval(value: i64) -> DateTimeInterval {
        DateTimeInterval::Month(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct DayKey;

impl IntervalKeyOption for DayKey {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        super::utils::day::to_date_time(value)
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::utils::day::to_value(src)
    }

    fn to_dt_interval(value: i64) -> DateTimeInterval {
        DateTimeInterval::Day(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct HourKey;

impl IntervalKeyOption for HourKey {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        super::utils::hour::to_date_time(value)
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::utils::hour::to_value(src)
    }

    fn to_dt_interval(value: i64) -> DateTimeInterval {
        DateTimeInterval::Hour(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct MinuteKey;

impl IntervalKeyOption for MinuteKey {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        super::utils::minute::to_date_time(value)
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::utils::minute::to_value(src)
    }

    fn to_dt_interval(value: i64) -> DateTimeInterval {
        DateTimeInterval::Minute(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Minute5Key;

impl IntervalKeyOption for Minute5Key {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        super::utils::min5::to_date_time(value)
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::utils::min5::to_value(src)
    }

    fn to_dt_interval(value: i64) -> DateTimeInterval {
        DateTimeInterval::Min5(value)
    }
}
