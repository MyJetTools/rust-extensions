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
        super::interval_utils::year::to_date_time(value)
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::interval_utils::year::to_value(src)
    }

    fn to_dt_interval(value: i64) -> DateTimeInterval {
        DateTimeInterval::Year(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct MonthKey;

impl IntervalKeyOption for MonthKey {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        super::interval_utils::month::to_date_time(value)
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::interval_utils::month::to_value(src)
    }

    fn to_dt_interval(value: i64) -> DateTimeInterval {
        DateTimeInterval::Month(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct DayKey;

impl IntervalKeyOption for DayKey {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        super::interval_utils::day::to_date_time(value)
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::interval_utils::day::to_value(src)
    }

    fn to_dt_interval(value: i64) -> DateTimeInterval {
        DateTimeInterval::Day(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct WeekMondayKey;

impl IntervalKeyOption for WeekMondayKey {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        super::interval_utils::week_monday::to_date_time(value)
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::interval_utils::week_monday::to_value(src)
    }

    fn to_dt_interval(value: i64) -> DateTimeInterval {
        DateTimeInterval::WeekMonday(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct WeekSundayKey;

impl IntervalKeyOption for WeekSundayKey {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        super::interval_utils::week_sunday::to_date_time(value)
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::interval_utils::week_sunday::to_value(src)
    }

    fn to_dt_interval(value: i64) -> DateTimeInterval {
        DateTimeInterval::WeekSunday(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct HourKey;

impl IntervalKeyOption for HourKey {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        super::interval_utils::hour::to_date_time(value)
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::interval_utils::hour::to_value(src)
    }

    fn to_dt_interval(value: i64) -> DateTimeInterval {
        DateTimeInterval::Hour(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Hour2Key;

impl IntervalKeyOption for Hour2Key {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        super::interval_utils::hour2::to_date_time(value)
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::interval_utils::hour2::to_value(src)
    }

    fn to_dt_interval(value: i64) -> DateTimeInterval {
        DateTimeInterval::Hour2(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Hour4Key;

impl IntervalKeyOption for Hour4Key {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        super::interval_utils::hour4::to_date_time(value)
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::interval_utils::hour4::to_value(src)
    }

    fn to_dt_interval(value: i64) -> DateTimeInterval {
        DateTimeInterval::Hour4(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct MinuteKey;

impl IntervalKeyOption for MinuteKey {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        super::interval_utils::minute::to_date_time(value)
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::interval_utils::minute::to_value(src)
    }

    fn to_dt_interval(value: i64) -> DateTimeInterval {
        DateTimeInterval::Minute(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Minute5Key;

impl IntervalKeyOption for Minute5Key {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        super::interval_utils::min5::to_date_time(value)
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::interval_utils::min5::to_value(src)
    }

    fn to_dt_interval(value: i64) -> DateTimeInterval {
        DateTimeInterval::Min5(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Minute15Key;

impl IntervalKeyOption for Minute15Key {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        super::interval_utils::min15::to_date_time(value)
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::interval_utils::min15::to_value(src)
    }

    fn to_dt_interval(value: i64) -> DateTimeInterval {
        DateTimeInterval::Min15(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Minute30Key;

impl IntervalKeyOption for Minute30Key {
    fn to_date_time(value: i64) -> Result<DateTimeAsMicroseconds, String> {
        super::interval_utils::min30::to_date_time(value)
    }

    fn to_value(src: DateTimeAsMicroseconds) -> i64 {
        super::interval_utils::min30::to_value(src)
    }

    fn to_dt_interval(value: i64) -> DateTimeInterval {
        DateTimeInterval::Min30(value)
    }
}
