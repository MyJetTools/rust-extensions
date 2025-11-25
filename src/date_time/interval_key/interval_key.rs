use std::time::Duration;

use crate::date_time::DateTimeAsMicroseconds;

use super::{IntervalKeyOption, *};

// Hour key formatted YYYYMMDDHH

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IntervalKey<TOption: IntervalKeyOption + Copy + Clone> {
    value: i64,
    _phantom: std::marker::PhantomData<TOption>,
}

impl<TOption: IntervalKeyOption + Copy + Clone> std::fmt::Debug for IntervalKey<TOption> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IntervalKey")
            .field("value", &self.value)
            .finish()
    }
}

impl<TOption: IntervalKeyOption + Clone + Copy> IntervalKey<TOption> {
    pub fn new(src: DateTimeAsMicroseconds) -> Self {
        Self {
            value: TOption::to_value(src),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn from_i64(value: i64) -> Self {
        Self {
            value,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn to_i64(&self) -> i64 {
        self.value
    }

    pub fn as_i64_ref(&self) -> &i64 {
        &self.value
    }

    pub fn to_interval_value(&self) -> IntervalKeyValue {
        TOption::to_interval_value(self.value)
    }

    pub fn add(&self, duration: Duration) -> Self {
        let dt: DateTimeAsMicroseconds = TOption::to_date_time(self.value).unwrap();
        let dt = dt.add(duration);
        Self {
            value: TOption::to_value(dt),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn sub(&self, duration: Duration) -> Self {
        let dt: DateTimeAsMicroseconds = TOption::to_date_time(self.value).unwrap();
        let dt = dt.add(duration);
        Self {
            value: TOption::to_value(dt),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn try_to_date_time(&self) -> Result<DateTimeAsMicroseconds, String> {
        TOption::to_date_time(self.value)
    }
}

impl Into<IntervalKey<YearKey>> for i64 {
    fn into(self) -> IntervalKey<YearKey> {
        IntervalKey::from_i64(self)
    }
}

impl Into<IntervalKey<YearKey>> for &'_ i64 {
    fn into(self) -> IntervalKey<YearKey> {
        IntervalKey::from_i64(*self)
    }
}

impl Into<IntervalKey<YearKey>> for u64 {
    fn into(self) -> IntervalKey<YearKey> {
        IntervalKey::from_i64(self as i64)
    }
}

impl Into<IntervalKey<YearKey>> for &'_ u64 {
    fn into(self) -> IntervalKey<YearKey> {
        IntervalKey::from_i64(*self as i64)
    }
}

impl Into<IntervalKey<MonthKey>> for i64 {
    fn into(self) -> IntervalKey<MonthKey> {
        IntervalKey::from_i64(self)
    }
}

impl Into<IntervalKey<MonthKey>> for &'_ i64 {
    fn into(self) -> IntervalKey<MonthKey> {
        IntervalKey::from_i64(*self)
    }
}

impl Into<IntervalKey<MonthKey>> for u64 {
    fn into(self) -> IntervalKey<MonthKey> {
        IntervalKey::from_i64(self as i64)
    }
}

impl Into<IntervalKey<MonthKey>> for &'_ u64 {
    fn into(self) -> IntervalKey<MonthKey> {
        IntervalKey::from_i64(*self as i64)
    }
}

impl Into<IntervalKey<DayKey>> for i64 {
    fn into(self) -> IntervalKey<DayKey> {
        IntervalKey::from_i64(self)
    }
}

impl Into<IntervalKey<DayKey>> for &'_ i64 {
    fn into(self) -> IntervalKey<DayKey> {
        IntervalKey::from_i64(*self)
    }
}

impl Into<IntervalKey<DayKey>> for u64 {
    fn into(self) -> IntervalKey<DayKey> {
        IntervalKey::from_i64(self as i64)
    }
}

impl Into<IntervalKey<DayKey>> for &'_ u64 {
    fn into(self) -> IntervalKey<DayKey> {
        IntervalKey::from_i64(*self as i64)
    }
}

impl Into<IntervalKey<HourKey>> for i64 {
    fn into(self) -> IntervalKey<HourKey> {
        IntervalKey::from_i64(self)
    }
}

impl Into<IntervalKey<HourKey>> for &'_ i64 {
    fn into(self) -> IntervalKey<HourKey> {
        IntervalKey::from_i64(*self)
    }
}

impl Into<IntervalKey<HourKey>> for u64 {
    fn into(self) -> IntervalKey<HourKey> {
        IntervalKey::from_i64(self as i64)
    }
}

impl Into<IntervalKey<HourKey>> for &'_ u64 {
    fn into(self) -> IntervalKey<HourKey> {
        IntervalKey::from_i64(*self as i64)
    }
}

impl Into<IntervalKey<MinuteKey>> for i64 {
    fn into(self) -> IntervalKey<MinuteKey> {
        IntervalKey::from_i64(self)
    }
}

impl Into<IntervalKey<MinuteKey>> for &'_ i64 {
    fn into(self) -> IntervalKey<MinuteKey> {
        IntervalKey::from_i64(*self)
    }
}

impl Into<IntervalKey<MinuteKey>> for u64 {
    fn into(self) -> IntervalKey<MinuteKey> {
        IntervalKey::from_i64(self as i64)
    }
}

impl Into<IntervalKey<MinuteKey>> for &'_ u64 {
    fn into(self) -> IntervalKey<MinuteKey> {
        IntervalKey::from_i64(*self as i64)
    }
}

impl Into<IntervalKey<Minute5Key>> for i64 {
    fn into(self) -> IntervalKey<Minute5Key> {
        IntervalKey::from_i64(self)
    }
}

impl TryInto<IntervalKey<Minute5Key>> for IntervalKey<MinuteKey> {
    type Error = String;
    fn try_into(self) -> Result<IntervalKey<Minute5Key>, Self::Error> {
        let dt: DateTimeAsMicroseconds = self.try_to_date_time()?;
        let result: IntervalKey<Minute5Key> = dt.into();
        Ok(result)
    }
}

impl TryInto<IntervalKey<MinuteKey>> for IntervalKey<Minute5Key> {
    type Error = String;
    fn try_into(self) -> Result<IntervalKey<MinuteKey>, Self::Error> {
        let dt: DateTimeAsMicroseconds = self.try_to_date_time()?;
        let result: IntervalKey<MinuteKey> = dt.into();
        Ok(result)
    }
}

impl Into<IntervalKey<Minute5Key>> for &'_ i64 {
    fn into(self) -> IntervalKey<Minute5Key> {
        IntervalKey::from_i64(*self)
    }
}

impl Into<IntervalKey<Minute5Key>> for u64 {
    fn into(self) -> IntervalKey<Minute5Key> {
        IntervalKey::from_i64(self as i64)
    }
}

impl Into<IntervalKey<Minute5Key>> for &'_ u64 {
    fn into(self) -> IntervalKey<Minute5Key> {
        IntervalKey::from_i64(*self as i64)
    }
}

impl Into<IntervalKey<YearKey>> for DateTimeAsMicroseconds {
    fn into(self) -> IntervalKey<YearKey> {
        IntervalKey::new(self)
    }
}

impl Into<IntervalKey<MonthKey>> for DateTimeAsMicroseconds {
    fn into(self) -> IntervalKey<MonthKey> {
        IntervalKey::new(self)
    }
}

impl Into<IntervalKey<DayKey>> for DateTimeAsMicroseconds {
    fn into(self) -> IntervalKey<DayKey> {
        IntervalKey::new(self)
    }
}

impl Into<IntervalKey<HourKey>> for DateTimeAsMicroseconds {
    fn into(self) -> IntervalKey<HourKey> {
        IntervalKey::new(self)
    }
}

impl Into<IntervalKey<MinuteKey>> for DateTimeAsMicroseconds {
    fn into(self) -> IntervalKey<MinuteKey> {
        IntervalKey::new(self)
    }
}

impl TryInto<DateTimeAsMicroseconds> for IntervalKey<YearKey> {
    type Error = String;
    fn try_into(self) -> Result<DateTimeAsMicroseconds, Self::Error> {
        YearKey::to_date_time(self.value)
    }
}

impl TryInto<DateTimeAsMicroseconds> for IntervalKey<MonthKey> {
    type Error = String;
    fn try_into(self) -> Result<DateTimeAsMicroseconds, Self::Error> {
        MonthKey::to_date_time(self.value)
    }
}

impl TryInto<DateTimeAsMicroseconds> for IntervalKey<DayKey> {
    type Error = String;
    fn try_into(self) -> Result<DateTimeAsMicroseconds, Self::Error> {
        DayKey::to_date_time(self.value)
    }
}

impl TryInto<DateTimeAsMicroseconds> for IntervalKey<HourKey> {
    type Error = String;
    fn try_into(self) -> Result<DateTimeAsMicroseconds, Self::Error> {
        HourKey::to_date_time(self.value)
    }
}

impl TryInto<DateTimeAsMicroseconds> for IntervalKey<MinuteKey> {
    type Error = String;
    fn try_into(self) -> Result<DateTimeAsMicroseconds, Self::Error> {
        MinuteKey::to_date_time(self.value)
    }
}

impl TryInto<DateTimeAsMicroseconds> for IntervalKey<Minute5Key> {
    type Error = String;
    fn try_into(self) -> Result<DateTimeAsMicroseconds, Self::Error> {
        Minute5Key::to_date_time(self.value)
    }
}

impl Into<IntervalKey<Minute5Key>> for DateTimeAsMicroseconds {
    fn into(self) -> IntervalKey<Minute5Key> {
        IntervalKey::new(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::date_time::*;

    #[test]
    fn test_year_key() {
        let d = DateTimeAsMicroseconds::from_str("2021-03-05T01:12:32.000000Z").unwrap();

        let year_key: super::IntervalKey<YearKey> = d.into();

        assert_eq!(year_key.value, 2021);

        let d_result: DateTimeAsMicroseconds = year_key.try_into().unwrap();

        assert_eq!("2021-01-01T00:00:00", &d_result.to_rfc3339()[..19]);
    }

    #[test]
    fn test_month_key() {
        let d = DateTimeAsMicroseconds::from_str("2021-03-05T01:12:32.000000Z").unwrap();

        let month_key: super::IntervalKey<MonthKey> = d.into();

        assert_eq!(month_key.value, 202103);

        let d_result: DateTimeAsMicroseconds = month_key.try_into().unwrap();

        assert_eq!("2021-03-01T00:00:00", &d_result.to_rfc3339()[..19]);
    }

    #[test]
    fn test_day_key() {
        let d = DateTimeAsMicroseconds::from_str("2021-03-05T01:12:32.000000Z").unwrap();

        let day_key: super::IntervalKey<DayKey> = d.into();

        assert_eq!(day_key.value, 20210305);

        let d_result: DateTimeAsMicroseconds = day_key.try_into().unwrap();

        assert_eq!("2021-03-05T00:00:00", &d_result.to_rfc3339()[..19]);
    }

    #[test]
    fn test_hour_key() {
        let d = DateTimeAsMicroseconds::from_str("2021-03-05T01:12:32.000000Z").unwrap();

        let hour_key: super::IntervalKey<HourKey> = d.into();

        assert_eq!(hour_key.value, 2021030501);

        let d_result: DateTimeAsMicroseconds = hour_key.try_into().unwrap();

        assert_eq!("2021-03-05T01:00:00", &d_result.to_rfc3339()[..19]);
    }

    #[test]
    fn test_minute_key() {
        let d = DateTimeAsMicroseconds::from_str("2021-03-05T01:12:32.000000Z").unwrap();

        let minute_key: super::IntervalKey<MinuteKey> = d.into();

        assert_eq!(minute_key.value, 202103050112);

        let d_result: DateTimeAsMicroseconds = minute_key.try_into().unwrap();

        assert_eq!("2021-03-05T01:12:00", &d_result.to_rfc3339()[..19]);
    }

    #[test]
    fn test_minute_five_key() {
        let d = DateTimeAsMicroseconds::from_str("2021-03-05T01:12:32.000000Z").unwrap();

        let minute_key: super::IntervalKey<Minute5Key> = d.into();

        assert_eq!(minute_key.value, 202103050110);

        let d_result: DateTimeAsMicroseconds = minute_key.try_into().unwrap();

        assert_eq!("2021-03-05T01:10:00", &d_result.to_rfc3339()[..19]);
    }
}
