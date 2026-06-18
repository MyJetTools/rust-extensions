use std::time::Duration;

use crate::date_time::DateTimeAsMicroseconds;

use super::{IntervalKeyOption, *};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

    /// Wraps a raw `i64` as a key **without validation**. The value must be a
    /// correctly-encoded key of this exact type — normally one previously produced
    /// by [`Self::to_i64`] / [`Self::new`] for the same `TOption`. A value encoded
    /// for a different key type (e.g. a minute key fed to a `DayKey`) silently
    /// mis-decodes. Off-slot values are normalized to the slot start on decode.
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

    pub fn to_dt_interval(&self) -> DateTimeInterval {
        TOption::to_dt_interval(self.value)
    }

    /// Shifts the key by `duration` (applied to the slot-start timestamp) and
    /// re-snaps to the slot start.
    ///
    /// This is **not** slot-aware: a `duration` smaller than one slot width
    /// returns the **same** key — e.g. for a week key
    /// `key.add(Duration::from_secs(60))` is a no-op; advance by a full week to
    /// reach the next bucket. To step exactly one bucket, add the slot's own width.
    pub fn add(&self, duration: Duration) -> Self {
        let dt: DateTimeAsMicroseconds = TOption::to_date_time(self.value).unwrap();
        let dt = dt.add(duration);
        Self {
            value: TOption::to_value(dt),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Shifts the key back by `duration` and re-snaps to the slot start. Like
    /// [`Self::add`], this is not slot-aware (see its note).
    pub fn sub(&self, duration: Duration) -> Self {
        let dt: DateTimeAsMicroseconds = TOption::to_date_time(self.value).unwrap();
        let dt = dt.sub(duration);
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

impl Into<IntervalKey<Minute15Key>> for i64 {
    fn into(self) -> IntervalKey<Minute15Key> {
        IntervalKey::from_i64(self)
    }
}

impl TryInto<IntervalKey<Minute15Key>> for IntervalKey<MinuteKey> {
    type Error = String;
    fn try_into(self) -> Result<IntervalKey<Minute15Key>, Self::Error> {
        let dt: DateTimeAsMicroseconds = self.try_to_date_time()?;
        let result: IntervalKey<Minute15Key> = dt.into();
        Ok(result)
    }
}

impl TryInto<IntervalKey<MinuteKey>> for IntervalKey<Minute15Key> {
    type Error = String;
    fn try_into(self) -> Result<IntervalKey<MinuteKey>, Self::Error> {
        let dt: DateTimeAsMicroseconds = self.try_to_date_time()?;
        let result: IntervalKey<MinuteKey> = dt.into();
        Ok(result)
    }
}

impl TryInto<IntervalKey<Minute15Key>> for IntervalKey<Minute5Key> {
    type Error = String;
    fn try_into(self) -> Result<IntervalKey<Minute15Key>, Self::Error> {
        let dt: DateTimeAsMicroseconds = self.try_to_date_time()?;
        let result: IntervalKey<Minute15Key> = dt.into();
        Ok(result)
    }
}

impl TryInto<IntervalKey<Minute5Key>> for IntervalKey<Minute15Key> {
    type Error = String;
    fn try_into(self) -> Result<IntervalKey<Minute5Key>, Self::Error> {
        let dt: DateTimeAsMicroseconds = self.try_to_date_time()?;
        let result: IntervalKey<Minute5Key> = dt.into();
        Ok(result)
    }
}

impl Into<IntervalKey<Minute15Key>> for &'_ i64 {
    fn into(self) -> IntervalKey<Minute15Key> {
        IntervalKey::from_i64(*self)
    }
}

impl Into<IntervalKey<Minute15Key>> for u64 {
    fn into(self) -> IntervalKey<Minute15Key> {
        IntervalKey::from_i64(self as i64)
    }
}

impl Into<IntervalKey<Minute15Key>> for &'_ u64 {
    fn into(self) -> IntervalKey<Minute15Key> {
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

impl TryInto<DateTimeAsMicroseconds> for IntervalKey<Minute15Key> {
    type Error = String;
    fn try_into(self) -> Result<DateTimeAsMicroseconds, Self::Error> {
        Minute15Key::to_date_time(self.value)
    }
}

impl Into<IntervalKey<Minute15Key>> for DateTimeAsMicroseconds {
    fn into(self) -> IntervalKey<Minute15Key> {
        IntervalKey::new(self)
    }
}

impl Into<IntervalKey<Minute30Key>> for i64 {
    fn into(self) -> IntervalKey<Minute30Key> {
        IntervalKey::from_i64(self)
    }
}

impl Into<IntervalKey<Minute30Key>> for &'_ i64 {
    fn into(self) -> IntervalKey<Minute30Key> {
        IntervalKey::from_i64(*self)
    }
}

impl Into<IntervalKey<Minute30Key>> for u64 {
    fn into(self) -> IntervalKey<Minute30Key> {
        IntervalKey::from_i64(self as i64)
    }
}

impl Into<IntervalKey<Minute30Key>> for &'_ u64 {
    fn into(self) -> IntervalKey<Minute30Key> {
        IntervalKey::from_i64(*self as i64)
    }
}

impl Into<IntervalKey<Minute30Key>> for DateTimeAsMicroseconds {
    fn into(self) -> IntervalKey<Minute30Key> {
        IntervalKey::new(self)
    }
}

impl TryInto<DateTimeAsMicroseconds> for IntervalKey<Minute30Key> {
    type Error = String;
    fn try_into(self) -> Result<DateTimeAsMicroseconds, Self::Error> {
        Minute30Key::to_date_time(self.value)
    }
}

// Cross-conversions within the minute family, mirroring Minute/Minute5/Minute15.
impl TryInto<IntervalKey<Minute30Key>> for IntervalKey<MinuteKey> {
    type Error = String;
    fn try_into(self) -> Result<IntervalKey<Minute30Key>, Self::Error> {
        let dt: DateTimeAsMicroseconds = self.try_to_date_time()?;
        let result: IntervalKey<Minute30Key> = dt.into();
        Ok(result)
    }
}

impl TryInto<IntervalKey<MinuteKey>> for IntervalKey<Minute30Key> {
    type Error = String;
    fn try_into(self) -> Result<IntervalKey<MinuteKey>, Self::Error> {
        let dt: DateTimeAsMicroseconds = self.try_to_date_time()?;
        let result: IntervalKey<MinuteKey> = dt.into();
        Ok(result)
    }
}

impl TryInto<IntervalKey<Minute30Key>> for IntervalKey<Minute5Key> {
    type Error = String;
    fn try_into(self) -> Result<IntervalKey<Minute30Key>, Self::Error> {
        let dt: DateTimeAsMicroseconds = self.try_to_date_time()?;
        let result: IntervalKey<Minute30Key> = dt.into();
        Ok(result)
    }
}

impl TryInto<IntervalKey<Minute5Key>> for IntervalKey<Minute30Key> {
    type Error = String;
    fn try_into(self) -> Result<IntervalKey<Minute5Key>, Self::Error> {
        let dt: DateTimeAsMicroseconds = self.try_to_date_time()?;
        let result: IntervalKey<Minute5Key> = dt.into();
        Ok(result)
    }
}

impl TryInto<IntervalKey<Minute30Key>> for IntervalKey<Minute15Key> {
    type Error = String;
    fn try_into(self) -> Result<IntervalKey<Minute30Key>, Self::Error> {
        let dt: DateTimeAsMicroseconds = self.try_to_date_time()?;
        let result: IntervalKey<Minute30Key> = dt.into();
        Ok(result)
    }
}

impl TryInto<IntervalKey<Minute15Key>> for IntervalKey<Minute30Key> {
    type Error = String;
    fn try_into(self) -> Result<IntervalKey<Minute15Key>, Self::Error> {
        let dt: DateTimeAsMicroseconds = self.try_to_date_time()?;
        let result: IntervalKey<Minute15Key> = dt.into();
        Ok(result)
    }
}

impl Into<IntervalKey<Hour2Key>> for i64 {
    fn into(self) -> IntervalKey<Hour2Key> {
        IntervalKey::from_i64(self)
    }
}

impl Into<IntervalKey<Hour2Key>> for &'_ i64 {
    fn into(self) -> IntervalKey<Hour2Key> {
        IntervalKey::from_i64(*self)
    }
}

impl Into<IntervalKey<Hour2Key>> for u64 {
    fn into(self) -> IntervalKey<Hour2Key> {
        IntervalKey::from_i64(self as i64)
    }
}

impl Into<IntervalKey<Hour2Key>> for &'_ u64 {
    fn into(self) -> IntervalKey<Hour2Key> {
        IntervalKey::from_i64(*self as i64)
    }
}

impl Into<IntervalKey<Hour2Key>> for DateTimeAsMicroseconds {
    fn into(self) -> IntervalKey<Hour2Key> {
        IntervalKey::new(self)
    }
}

impl TryInto<DateTimeAsMicroseconds> for IntervalKey<Hour2Key> {
    type Error = String;
    fn try_into(self) -> Result<DateTimeAsMicroseconds, Self::Error> {
        Hour2Key::to_date_time(self.value)
    }
}

impl Into<IntervalKey<Hour4Key>> for i64 {
    fn into(self) -> IntervalKey<Hour4Key> {
        IntervalKey::from_i64(self)
    }
}

impl Into<IntervalKey<Hour4Key>> for &'_ i64 {
    fn into(self) -> IntervalKey<Hour4Key> {
        IntervalKey::from_i64(*self)
    }
}

impl Into<IntervalKey<Hour4Key>> for u64 {
    fn into(self) -> IntervalKey<Hour4Key> {
        IntervalKey::from_i64(self as i64)
    }
}

impl Into<IntervalKey<Hour4Key>> for &'_ u64 {
    fn into(self) -> IntervalKey<Hour4Key> {
        IntervalKey::from_i64(*self as i64)
    }
}

impl Into<IntervalKey<Hour4Key>> for DateTimeAsMicroseconds {
    fn into(self) -> IntervalKey<Hour4Key> {
        IntervalKey::new(self)
    }
}

impl TryInto<DateTimeAsMicroseconds> for IntervalKey<Hour4Key> {
    type Error = String;
    fn try_into(self) -> Result<DateTimeAsMicroseconds, Self::Error> {
        Hour4Key::to_date_time(self.value)
    }
}

impl Into<IntervalKey<WeekMondayKey>> for i64 {
    fn into(self) -> IntervalKey<WeekMondayKey> {
        IntervalKey::from_i64(self)
    }
}

impl Into<IntervalKey<WeekMondayKey>> for &'_ i64 {
    fn into(self) -> IntervalKey<WeekMondayKey> {
        IntervalKey::from_i64(*self)
    }
}

impl Into<IntervalKey<WeekMondayKey>> for u64 {
    fn into(self) -> IntervalKey<WeekMondayKey> {
        IntervalKey::from_i64(self as i64)
    }
}

impl Into<IntervalKey<WeekMondayKey>> for &'_ u64 {
    fn into(self) -> IntervalKey<WeekMondayKey> {
        IntervalKey::from_i64(*self as i64)
    }
}

impl Into<IntervalKey<WeekMondayKey>> for DateTimeAsMicroseconds {
    fn into(self) -> IntervalKey<WeekMondayKey> {
        IntervalKey::new(self)
    }
}

impl TryInto<DateTimeAsMicroseconds> for IntervalKey<WeekMondayKey> {
    type Error = String;
    fn try_into(self) -> Result<DateTimeAsMicroseconds, Self::Error> {
        WeekMondayKey::to_date_time(self.value)
    }
}

impl Into<IntervalKey<WeekSundayKey>> for i64 {
    fn into(self) -> IntervalKey<WeekSundayKey> {
        IntervalKey::from_i64(self)
    }
}

impl Into<IntervalKey<WeekSundayKey>> for &'_ i64 {
    fn into(self) -> IntervalKey<WeekSundayKey> {
        IntervalKey::from_i64(*self)
    }
}

impl Into<IntervalKey<WeekSundayKey>> for u64 {
    fn into(self) -> IntervalKey<WeekSundayKey> {
        IntervalKey::from_i64(self as i64)
    }
}

impl Into<IntervalKey<WeekSundayKey>> for &'_ u64 {
    fn into(self) -> IntervalKey<WeekSundayKey> {
        IntervalKey::from_i64(*self as i64)
    }
}

impl Into<IntervalKey<WeekSundayKey>> for DateTimeAsMicroseconds {
    fn into(self) -> IntervalKey<WeekSundayKey> {
        IntervalKey::new(self)
    }
}

impl TryInto<DateTimeAsMicroseconds> for IntervalKey<WeekSundayKey> {
    type Error = String;
    fn try_into(self) -> Result<DateTimeAsMicroseconds, Self::Error> {
        WeekSundayKey::to_date_time(self.value)
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

    #[test]
    fn test_minute_fifteen_key() {
        let d = DateTimeAsMicroseconds::from_str("2021-03-05T01:22:32.000000Z").unwrap();

        let minute_key: super::IntervalKey<Minute15Key> = d.into();

        assert_eq!(minute_key.value, 202103050115);

        let d_result: DateTimeAsMicroseconds = minute_key.try_into().unwrap();

        assert_eq!("2021-03-05T01:15:00", &d_result.to_rfc3339()[..19]);
    }

    #[test]
    fn test_minute_thirty_key() {
        let d = DateTimeAsMicroseconds::from_str("2021-03-05T01:42:32.000000Z").unwrap();

        let minute_key: super::IntervalKey<Minute30Key> = d.into();

        assert_eq!(minute_key.value, 202103050130);

        let d_result: DateTimeAsMicroseconds = minute_key.try_into().unwrap();

        assert_eq!("2021-03-05T01:30:00", &d_result.to_rfc3339()[..19]);
    }

    #[test]
    fn test_hour_two_key() {
        let d = DateTimeAsMicroseconds::from_str("2021-03-05T03:12:32.000000Z").unwrap();

        let hour_key: super::IntervalKey<Hour2Key> = d.into();

        assert_eq!(hour_key.value, 2021030502);

        let d_result: DateTimeAsMicroseconds = hour_key.try_into().unwrap();

        assert_eq!("2021-03-05T02:00:00", &d_result.to_rfc3339()[..19]);
    }

    #[test]
    fn test_hour_four_key() {
        let d = DateTimeAsMicroseconds::from_str("2021-03-05T13:12:32.000000Z").unwrap();

        let hour_key: super::IntervalKey<Hour4Key> = d.into();

        assert_eq!(hour_key.value, 2021030512);

        let d_result: DateTimeAsMicroseconds = hour_key.try_into().unwrap();

        assert_eq!("2021-03-05T12:00:00", &d_result.to_rfc3339()[..19]);
    }

    #[test]
    fn test_week_monday_key() {
        // 2021-03-05 is a Friday; its Monday-started week begins 2021-03-01.
        let d = DateTimeAsMicroseconds::from_str("2021-03-05T13:12:32.000000Z").unwrap();

        let week_key: super::IntervalKey<WeekMondayKey> = d.into();

        assert_eq!(week_key.value, 20210301);

        let d_result: DateTimeAsMicroseconds = week_key.try_into().unwrap();

        assert_eq!("2021-03-01T00:00:00", &d_result.to_rfc3339()[..19]);
    }

    #[test]
    fn test_week_sunday_key() {
        // 2021-03-05 is a Friday; its Sunday-started week begins 2021-02-28.
        let d = DateTimeAsMicroseconds::from_str("2021-03-05T13:12:32.000000Z").unwrap();

        let week_key: super::IntervalKey<WeekSundayKey> = d.into();

        assert_eq!(week_key.value, 20210228);

        let d_result: DateTimeAsMicroseconds = week_key.try_into().unwrap();

        assert_eq!("2021-02-28T00:00:00", &d_result.to_rfc3339()[..19]);
    }

    #[test]
    fn test_week_keys_on_boundary_days() {
        // 2021-03-01 is a Monday, 2021-03-07 is a Sunday - the two week ends.
        let monday = DateTimeAsMicroseconds::from_str("2021-03-01T00:00:00.000000Z").unwrap();
        let sunday = DateTimeAsMicroseconds::from_str("2021-03-07T23:59:59.000000Z").unwrap();

        // Both days fall into the same Monday-started week (2021-03-01).
        let m: super::IntervalKey<WeekMondayKey> = monday.into();
        let s: super::IntervalKey<WeekMondayKey> = sunday.into();
        assert_eq!(m.value, 20210301);
        assert_eq!(s.value, 20210301);

        // Sunday-started: the Monday belongs to the week that began 2021-02-28,
        // the Sunday starts its own week (2021-03-07).
        let m: super::IntervalKey<WeekSundayKey> = monday.into();
        let s: super::IntervalKey<WeekSundayKey> = sunday.into();
        assert_eq!(m.value, 20210228);
        assert_eq!(s.value, 20210307);
    }

    #[test]
    fn test_off_slot_raw_value_is_normalized() {
        // minute 17 is not a valid slot start - it has to be cut to the slot
        let key: super::IntervalKey<Minute15Key> = super::IntervalKey::from_i64(202103050117);
        let d: DateTimeAsMicroseconds = key.try_to_date_time().unwrap();
        assert_eq!("2021-03-05T01:15:00", &d.to_rfc3339()[..19]);

        let key: super::IntervalKey<Minute5Key> = super::IntervalKey::from_i64(202103050117);
        let d: DateTimeAsMicroseconds = key.try_to_date_time().unwrap();
        assert_eq!("2021-03-05T01:15:00", &d.to_rfc3339()[..19]);

        // minute 42 cuts to the 30 slot
        let key: super::IntervalKey<Minute30Key> = super::IntervalKey::from_i64(202103050142);
        let d: DateTimeAsMicroseconds = key.try_to_date_time().unwrap();
        assert_eq!("2021-03-05T01:30:00", &d.to_rfc3339()[..19]);

        // hour 03 cuts to the 2-hour slot start (02)
        let key: super::IntervalKey<Hour2Key> = super::IntervalKey::from_i64(2021030503);
        let d: DateTimeAsMicroseconds = key.try_to_date_time().unwrap();
        assert_eq!("2021-03-05T02:00:00", &d.to_rfc3339()[..19]);

        // hour 13 cuts to the 4-hour slot start (12)
        let key: super::IntervalKey<Hour4Key> = super::IntervalKey::from_i64(2021030513);
        let d: DateTimeAsMicroseconds = key.try_to_date_time().unwrap();
        assert_eq!("2021-03-05T12:00:00", &d.to_rfc3339()[..19]);

        // 2021-03-05 (Friday) cuts back to its Monday-started week start (03-01)
        let key: super::IntervalKey<WeekMondayKey> = super::IntervalKey::from_i64(20210305);
        let d: DateTimeAsMicroseconds = key.try_to_date_time().unwrap();
        assert_eq!("2021-03-01T00:00:00", &d.to_rfc3339()[..19]);

        // ... and to its Sunday-started week start (02-28)
        let key: super::IntervalKey<WeekSundayKey> = super::IntervalKey::from_i64(20210305);
        let d: DateTimeAsMicroseconds = key.try_to_date_time().unwrap();
        assert_eq!("2021-02-28T00:00:00", &d.to_rfc3339()[..19]);
    }

    #[test]
    fn test_min5_min15_conversions() {
        let d = DateTimeAsMicroseconds::from_str("2021-03-05T01:22:32.000000Z").unwrap();

        let min5_key: super::IntervalKey<Minute5Key> = d.into();
        assert_eq!(min5_key.value, 202103050120);

        let min15_key: super::IntervalKey<Minute15Key> = min5_key.try_into().unwrap();
        assert_eq!(min15_key.value, 202103050115);

        let min5_key: super::IntervalKey<Minute5Key> = min15_key.try_into().unwrap();
        assert_eq!(min5_key.value, 202103050115);
    }

    #[test]
    fn test_date_time_interval_dispatch() {
        // Exercises the runtime DateTimeInterval enum: from_dt_to_*, to_i64, and
        // to_date_time — the hand-maintained 12-arm dispatch that the compiler
        // cannot check for wrong-module wiring.
        let d = DateTimeAsMicroseconds::from_str("2021-03-05T01:42:32.000000Z").unwrap();

        // from_dt_to_* + to_value wiring (variant & packed value).
        assert_eq!(DateTimeInterval::from_dt_to_year(d), DateTimeInterval::Year(2021));
        assert_eq!(DateTimeInterval::from_dt_to_month(d), DateTimeInterval::Month(202103));
        assert_eq!(DateTimeInterval::from_dt_to_day(d), DateTimeInterval::Day(20210305));
        assert_eq!(
            DateTimeInterval::from_dt_to_week_monday(d),
            DateTimeInterval::WeekMonday(20210301)
        );
        assert_eq!(
            DateTimeInterval::from_dt_to_week_sunday(d),
            DateTimeInterval::WeekSunday(20210228)
        );
        assert_eq!(DateTimeInterval::from_dt_to_hour(d), DateTimeInterval::Hour(2021030501));
        assert_eq!(DateTimeInterval::from_dt_to_hour2(d), DateTimeInterval::Hour2(2021030500));
        assert_eq!(DateTimeInterval::from_dt_to_hour4(d), DateTimeInterval::Hour4(2021030500));
        assert_eq!(
            DateTimeInterval::from_dt_to_minute(d),
            DateTimeInterval::Minute(202103050142)
        );
        assert_eq!(DateTimeInterval::from_dt_to_min5(d), DateTimeInterval::Min5(202103050140));
        assert_eq!(DateTimeInterval::from_dt_to_min15(d), DateTimeInterval::Min15(202103050130));
        assert_eq!(DateTimeInterval::from_dt_to_min30(d), DateTimeInterval::Min30(202103050130));

        // to_i64 arm (new variants).
        assert_eq!(DateTimeInterval::Min30(202103050130).to_i64(), 202103050130);
        assert_eq!(DateTimeInterval::Hour2(2021030500).to_i64(), 2021030500);
        assert_eq!(DateTimeInterval::Hour4(2021030500).to_i64(), 2021030500);
        assert_eq!(DateTimeInterval::WeekMonday(20210301).to_i64(), 20210301);
        assert_eq!(DateTimeInterval::WeekSunday(20210228).to_i64(), 20210228);

        // to_date_time arm (slot start) — pins each arm to the right utils module.
        let chk = |iv: DateTimeInterval, expected: &str| {
            assert_eq!(&iv.to_date_time().unwrap().to_rfc3339()[..19], expected, "{:?}", iv);
        };
        chk(DateTimeInterval::from_dt_to_year(d), "2021-01-01T00:00:00");
        chk(DateTimeInterval::from_dt_to_month(d), "2021-03-01T00:00:00");
        chk(DateTimeInterval::from_dt_to_day(d), "2021-03-05T00:00:00");
        chk(DateTimeInterval::from_dt_to_week_monday(d), "2021-03-01T00:00:00");
        chk(DateTimeInterval::from_dt_to_week_sunday(d), "2021-02-28T00:00:00");
        chk(DateTimeInterval::from_dt_to_hour(d), "2021-03-05T01:00:00");
        chk(DateTimeInterval::from_dt_to_hour2(d), "2021-03-05T00:00:00");
        chk(DateTimeInterval::from_dt_to_hour4(d), "2021-03-05T00:00:00");
        chk(DateTimeInterval::from_dt_to_minute(d), "2021-03-05T01:42:00");
        chk(DateTimeInterval::from_dt_to_min5(d), "2021-03-05T01:40:00");
        chk(DateTimeInterval::from_dt_to_min15(d), "2021-03-05T01:30:00");
        chk(DateTimeInterval::from_dt_to_min30(d), "2021-03-05T01:30:00");
    }

    #[test]
    fn test_to_dt_interval_and_cross_path() {
        // IntervalKey::to_dt_interval must map to the matching DateTimeInterval
        // variant and agree with the typed value and DateTimeInterval::from_dt_to_*.
        let d = DateTimeAsMicroseconds::from_str("2021-03-05T01:42:32.000000Z").unwrap();

        let h2: super::IntervalKey<Hour2Key> = d.into();
        assert_eq!(h2.to_dt_interval(), DateTimeInterval::Hour2(2021030500));
        assert_eq!(h2.to_dt_interval().to_i64(), h2.to_i64());
        assert_eq!(h2.to_dt_interval(), DateTimeInterval::from_dt_to_hour2(d));

        let h4: super::IntervalKey<Hour4Key> = d.into();
        assert_eq!(h4.to_dt_interval(), DateTimeInterval::Hour4(2021030500));
        assert_eq!(h4.to_dt_interval(), DateTimeInterval::from_dt_to_hour4(d));

        let m30: super::IntervalKey<Minute30Key> = d.into();
        assert_eq!(m30.to_dt_interval(), DateTimeInterval::Min30(202103050130));
        assert_eq!(m30.to_dt_interval(), DateTimeInterval::from_dt_to_min30(d));

        let wm: super::IntervalKey<WeekMondayKey> = d.into();
        assert_eq!(wm.to_dt_interval(), DateTimeInterval::WeekMonday(20210301));
        assert_eq!(wm.to_dt_interval(), DateTimeInterval::from_dt_to_week_monday(d));

        let ws: super::IntervalKey<WeekSundayKey> = d.into();
        assert_eq!(ws.to_dt_interval(), DateTimeInterval::WeekSunday(20210228));
        assert_eq!(ws.to_dt_interval(), DateTimeInterval::from_dt_to_week_sunday(d));
    }

    #[test]
    fn test_week_keys_across_boundaries() {
        // 2021-01-01 is a Friday; its week start falls in the previous YEAR.
        let d = DateTimeAsMicroseconds::from_str("2021-01-01T08:00:00.000000Z").unwrap();
        let wm: super::IntervalKey<WeekMondayKey> = d.into();
        let ws: super::IntervalKey<WeekSundayKey> = d.into();
        assert_eq!(wm.value, 20201228); // Mon 2020-12-28
        assert_eq!(ws.value, 20201227); // Sun 2020-12-27

        // 2020-01-01 is a Wednesday; Monday week start crosses the year boundary.
        let d = DateTimeAsMicroseconds::from_str("2020-01-01T00:00:00.000000Z").unwrap();
        let wm: super::IntervalKey<WeekMondayKey> = d.into();
        assert_eq!(wm.value, 20191230); // Mon 2019-12-30

        // Leap year: 2024-03-01 is a Friday; Monday week start crosses Feb 29.
        let d = DateTimeAsMicroseconds::from_str("2024-03-01T12:00:00.000000Z").unwrap();
        let wm: super::IntervalKey<WeekMondayKey> = d.into();
        assert_eq!(wm.value, 20240226); // Mon 2024-02-26
        let d_result: DateTimeAsMicroseconds = wm.try_into().unwrap();
        assert_eq!("2024-02-26T00:00:00", &d_result.to_rfc3339()[..19]);
    }

    #[test]
    fn test_week_start_lands_on_correct_weekday() {
        use crate::date_time::interval_key::interval_utils::{week_monday, week_sunday};
        use chrono::Datelike;

        for s in [
            "2021-03-01T00:00:00.000000Z", // Mon
            "2021-03-04T10:00:00.000000Z", // Thu
            "2021-03-07T23:59:59.000000Z", // Sun
            "2024-02-29T12:00:00.000000Z", // leap day (Thu)
            "2021-01-01T08:00:00.000000Z", // Fri (year boundary)
        ] {
            let d = DateTimeAsMicroseconds::from_str(s).unwrap();

            let wm = week_monday::to_week_start(d);
            assert_eq!(wm.to_chrono_utc().weekday(), chrono::Weekday::Mon, "monday start for {s}");

            let ws = week_sunday::to_week_start(d);
            assert_eq!(ws.to_chrono_utc().weekday(), chrono::Weekday::Sun, "sunday start for {s}");
        }
    }

    #[test]
    fn test_numeric_into_for_new_keys() {
        let m30: super::IntervalKey<Minute30Key> = 202103050130i64.into();
        assert_eq!(m30.to_i64(), 202103050130);
        let m30: super::IntervalKey<Minute30Key> = (&202103050130i64).into();
        assert_eq!(m30.to_i64(), 202103050130);
        let m30: super::IntervalKey<Minute30Key> = 202103050130u64.into();
        assert_eq!(m30.to_i64(), 202103050130);
        let m30: super::IntervalKey<Minute30Key> = (&202103050130u64).into();
        assert_eq!(m30.to_i64(), 202103050130);

        let h2: super::IntervalKey<Hour2Key> = 2021030500i64.into();
        assert_eq!(h2.to_i64(), 2021030500);
        let h4: super::IntervalKey<Hour4Key> = 2021030500u64.into();
        assert_eq!(h4.to_i64(), 2021030500);
        let wm: super::IntervalKey<WeekMondayKey> = 20210301i64.into();
        assert_eq!(wm.to_i64(), 20210301);
        let ws: super::IntervalKey<WeekSundayKey> = (&20210228u64).into();
        assert_eq!(ws.to_i64(), 20210228);
    }

    #[test]
    fn test_pre_1970_timestamps() {
        // 1969-12-31 22:37:15 UTC (negative unix micros). Wall-clock fields are
        // derived via chrono and stay non-negative, so slots floor toward the slot
        // start (not toward zero).
        let d = DateTimeAsMicroseconds::from_str("1969-12-31T22:37:15.000000Z").unwrap();

        let m30: super::IntervalKey<Minute30Key> = d.into();
        assert_eq!(m30.value, 196912312230); // min 37 -> slot 30

        let h2: super::IntervalKey<Hour2Key> = d.into();
        assert_eq!(h2.value, 1969123122); // hour 22 (even slot)

        // 1969-12-31 is a Wednesday.
        let wm: super::IntervalKey<WeekMondayKey> = d.into();
        assert_eq!(wm.value, 19691229); // Mon 1969-12-29
        let ws: super::IntervalKey<WeekSundayKey> = d.into();
        assert_eq!(ws.value, 19691228); // Sun 1969-12-28

        let back: DateTimeAsMicroseconds = m30.try_into().unwrap();
        assert_eq!("1969-12-31T22:30:00", &back.to_rfc3339()[..19]);
    }

    #[test]
    fn test_min30_cross_conversions() {
        let d = DateTimeAsMicroseconds::from_str("2021-03-05T01:42:32.000000Z").unwrap();

        let min30: super::IntervalKey<Minute30Key> = d.into();
        assert_eq!(min30.value, 202103050130);

        let minute: super::IntervalKey<MinuteKey> = d.into();
        let from_minute: super::IntervalKey<Minute30Key> = minute.try_into().unwrap();
        assert_eq!(from_minute.value, 202103050130);

        let min5: super::IntervalKey<Minute5Key> = min30.try_into().unwrap();
        assert_eq!(min5.value, 202103050130);

        let min15: super::IntervalKey<Minute15Key> = min30.try_into().unwrap();
        assert_eq!(min15.value, 202103050130);

        let back_to_min30: super::IntervalKey<Minute30Key> = min15.try_into().unwrap();
        assert_eq!(back_to_min30.value, 202103050130);

        let back_to_minute: super::IntervalKey<MinuteKey> = min30.try_into().unwrap();
        assert_eq!(back_to_minute.value, 202103050130);
    }
}
