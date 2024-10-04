use super::DateTimeAsMicroseconds;

// Hour key formatted YYYYMMDDHH

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HourKey(u32);

impl Into<HourKey> for DateTimeAsMicroseconds {
    fn into(self) -> HourKey {
        let date_time_struct: super::DateTimeStruct = self.into();

        let result = (date_time_struct.year as u32) * 1000000
            + date_time_struct.month * 10000
            + date_time_struct.day * 100
            + date_time_struct.time.hour;

        result.into()
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
