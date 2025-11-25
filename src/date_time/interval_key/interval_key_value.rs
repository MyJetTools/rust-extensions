use crate::date_time::DateTimeAsMicroseconds;

pub enum IntervalKeyValue {
    Minute(i64),
    Min5(i64),
    Hour(i64),
    Day(i64),
    Month(i64),
    Year(i64),
}

impl IntervalKeyValue {
    pub fn from_dt_to_minute(dt: DateTimeAsMicroseconds) -> Self {
        let value = super::utils::minute::to_value(dt);
        Self::Minute(value)
    }

    pub fn from_dt_to_min5(dt: DateTimeAsMicroseconds) -> Self {
        let value = super::utils::min5::to_value(dt);
        Self::Min5(value)
    }

    pub fn from_dt_to_hour(dt: DateTimeAsMicroseconds) -> Self {
        let value = super::utils::hour::to_value(dt);
        Self::Hour(value)
    }

    pub fn from_dt_to_day(dt: DateTimeAsMicroseconds) -> Self {
        let value = super::utils::day::to_value(dt);
        Self::Day(value)
    }

    pub fn from_dt_to_month(dt: DateTimeAsMicroseconds) -> Self {
        let value = super::utils::month::to_value(dt);
        Self::Month(value)
    }

    pub fn from_dt_to_year(dt: DateTimeAsMicroseconds) -> Self {
        let value = super::utils::year::to_value(dt);
        Self::Year(value)
    }

    pub fn to_date_time(&self) -> Result<DateTimeAsMicroseconds, String> {
        match self {
            Self::Minute(value) => super::utils::minute::to_date_time(*value),
            Self::Min5(value) => super::utils::min5::to_date_time(*value),
            Self::Hour(value) => super::utils::hour::to_date_time(*value),
            Self::Day(value) => super::utils::day::to_date_time(*value),
            Self::Month(value) => super::utils::month::to_date_time(*value),
            Self::Year(value) => super::utils::year::to_date_time(*value),
        }
    }

    pub fn to_i64(&self) -> i64 {
        match self {
            Self::Minute(value) => *value,
            Self::Min5(value) => *value,
            Self::Hour(value) => *value,
            Self::Day(value) => *value,
            Self::Month(value) => *value,
            Self::Year(value) => *value,
        }
    }
}
