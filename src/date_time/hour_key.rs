use std::time::Duration;

use super::DateTimeAsMicroseconds;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub trait IntervalKeyOption {
    fn to_date_time(&self) -> DateTimeAsMicroseconds;
}

// Hour key formatted YYYYMMDDHH

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntervalKey {
    Month(u32),
    Day(u32),
    Hour(u32),
    Min(u32),
}

impl IntervalKey {
    pub fn as_hour(value: u32) -> Self {
        Self::Hour(value)
    }

    pub fn to_u32(&self) -> u32 {
        match self {
            Self::Month(value) => *value,
            Self::Day(value) => *value,
            Self::Hour(value) => *value,
            Self::Min(value) => *value,
        }
    }

    pub fn add(&self, duration: Duration) -> Self {
        let dt: DateTimeAsMicroseconds = (*self).try_into().unwrap();
        let dt = dt.add(duration);
        dt.into()
    }

    pub fn sub(&self, duration: Duration) -> Self {
        let dt: DateTimeAsMicroseconds = (*self).try_into().unwrap();
        let dt = dt.add(duration);
        dt.into()
    }
}

impl TryInto<DateTimeAsMicroseconds> for HourKey {
    type Error = String;
    fn try_into(self) -> Result<DateTimeAsMicroseconds, Self::Error> {
        let year = self.0 / 1000000;
        let month = (self.0 % 1000000) / 10000;
        let day = (self.0 % 10000) / 100;
        let hour = self.0 % 100;

        let date_time_struct = super::DateTimeStruct {
            year: year as i32,
            month: month,
            day: day,
            time: super::TimeStruct {
                hour: hour,
                min: 0,
                sec: 0,
                micros: 0,
            },
            dow: None,
        };

        let result = date_time_struct.to_date_time_as_microseconds();

        match result {
            Some(value) => Ok(value),
            None => Err(format!("{} is not a hour key", self.0)),
        }
    }
}

impl Into<u32> for HourKey {
    fn into(self) -> u32 {
        self.0
    }
}

impl Into<HourKey> for u32 {
    fn into(self) -> HourKey {
        HourKey(self)
    }
}

impl Into<HourKey> for u64 {
    fn into(self) -> HourKey {
        HourKey(self as u32)
    }
}

impl Into<HourKey> for i64 {
    fn into(self) -> HourKey {
        HourKey(self as u32)
    }
}

#[cfg(test)]
mod tests {
    use crate::date_time::DateTimeAsMicroseconds;

    #[test]
    fn get_hour_key() {
        let d = DateTimeAsMicroseconds::from_str("2021-03-05T01:12:32.000000Z").unwrap();

        let hour_key: super::HourKey = d.into();

        assert_eq!(hour_key.0, 2021030501);

        let d_result: DateTimeAsMicroseconds = hour_key.try_into().unwrap();

        assert_eq!("2021-03-05T01:00:00", &d_result.to_rfc3339()[..19]);
    }
}
