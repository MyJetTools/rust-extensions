use crate::date_time::DateTimeAsMicroseconds;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DateTimeInterval {
    Minute(i64),
    Min5(i64),
    Min15(i64),
    Min30(i64),
    Hour(i64),
    Hour2(i64),
    Hour4(i64),
    Day(i64),
    WeekMonday(i64),
    WeekSunday(i64),
    Month(i64),
    Year(i64),
}

impl DateTimeInterval {
    pub fn from_dt_to_minute(dt: DateTimeAsMicroseconds) -> Self {
        let value = super::interval_utils::minute::to_value(dt);
        Self::Minute(value)
    }

    pub fn from_dt_to_min5(dt: DateTimeAsMicroseconds) -> Self {
        let value = super::interval_utils::min5::to_value(dt);
        Self::Min5(value)
    }

    pub fn from_dt_to_min15(dt: DateTimeAsMicroseconds) -> Self {
        let value = super::interval_utils::min15::to_value(dt);
        Self::Min15(value)
    }

    pub fn from_dt_to_min30(dt: DateTimeAsMicroseconds) -> Self {
        let value = super::interval_utils::min30::to_value(dt);
        Self::Min30(value)
    }

    pub fn from_dt_to_hour(dt: DateTimeAsMicroseconds) -> Self {
        let value = super::interval_utils::hour::to_value(dt);
        Self::Hour(value)
    }

    pub fn from_dt_to_hour2(dt: DateTimeAsMicroseconds) -> Self {
        let value = super::interval_utils::hour2::to_value(dt);
        Self::Hour2(value)
    }

    pub fn from_dt_to_hour4(dt: DateTimeAsMicroseconds) -> Self {
        let value = super::interval_utils::hour4::to_value(dt);
        Self::Hour4(value)
    }

    pub fn from_dt_to_day(dt: DateTimeAsMicroseconds) -> Self {
        let value = super::interval_utils::day::to_value(dt);
        Self::Day(value)
    }

    pub fn from_dt_to_week_monday(dt: DateTimeAsMicroseconds) -> Self {
        let value = super::interval_utils::week_monday::to_value(dt);
        Self::WeekMonday(value)
    }

    pub fn from_dt_to_week_sunday(dt: DateTimeAsMicroseconds) -> Self {
        let value = super::interval_utils::week_sunday::to_value(dt);
        Self::WeekSunday(value)
    }

    pub fn from_dt_to_month(dt: DateTimeAsMicroseconds) -> Self {
        let value = super::interval_utils::month::to_value(dt);
        Self::Month(value)
    }

    pub fn from_dt_to_year(dt: DateTimeAsMicroseconds) -> Self {
        let value = super::interval_utils::year::to_value(dt);
        Self::Year(value)
    }

    pub fn to_date_time(&self) -> Result<DateTimeAsMicroseconds, String> {
        match self {
            Self::Minute(value) => super::interval_utils::minute::to_date_time(*value),
            Self::Min5(value) => super::interval_utils::min5::to_date_time(*value),
            Self::Min15(value) => super::interval_utils::min15::to_date_time(*value),
            Self::Min30(value) => super::interval_utils::min30::to_date_time(*value),
            Self::Hour(value) => super::interval_utils::hour::to_date_time(*value),
            Self::Hour2(value) => super::interval_utils::hour2::to_date_time(*value),
            Self::Hour4(value) => super::interval_utils::hour4::to_date_time(*value),
            Self::Day(value) => super::interval_utils::day::to_date_time(*value),
            Self::WeekMonday(value) => super::interval_utils::week_monday::to_date_time(*value),
            Self::WeekSunday(value) => super::interval_utils::week_sunday::to_date_time(*value),
            Self::Month(value) => super::interval_utils::month::to_date_time(*value),
            Self::Year(value) => super::interval_utils::year::to_date_time(*value),
        }
    }

    pub fn to_i64(&self) -> i64 {
        match self {
            Self::Minute(value) => *value,
            Self::Min5(value) => *value,
            Self::Min15(value) => *value,
            Self::Min30(value) => *value,
            Self::Hour(value) => *value,
            Self::Hour2(value) => *value,
            Self::Hour4(value) => *value,
            Self::Day(value) => *value,
            Self::WeekMonday(value) => *value,
            Self::WeekSunday(value) => *value,
            Self::Month(value) => *value,
            Self::Year(value) => *value,
        }
    }
}
