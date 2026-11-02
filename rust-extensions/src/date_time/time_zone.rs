use super::DateTimeAsMicroseconds;

const MINUTES_IN_ONE_DAY: i32 = 24 * 60;

/// A timezone expressed purely as a fixed offset from UTC, stored in **minutes**.
///
/// `UTC+1` = `60`, `UTC-5` = `-300`, `UTC+5:45` (Nepal) = `345`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TimeZone {
    offset_in_minutes: i32,
}

impl TimeZone {
    pub const fn utc() -> Self {
        Self {
            offset_in_minutes: 0,
        }
    }

    pub const fn from_minutes(offset_in_minutes: i32) -> Self {
        Self { offset_in_minutes }
    }

    /// Derives the offset from a `server_time` (UTC) and the `local_time` the same moment
    /// reads as on the local clock (encoded as a plain [`DateTimeAsMicroseconds`]). The
    /// offset is `local_time - server_time`, rounded to the nearest 15 minutes — 15-minute
    /// steps exist in the wild (Nepal `UTC+5:45`, `UTC+8:45`), so a raw difference of `+58`
    /// minutes becomes `+60`, `+50` becomes `+45`, `+7` becomes `0`.
    pub fn from_server_and_local_time(
        server_time: DateTimeAsMicroseconds,
        local_time: DateTimeAsMicroseconds,
    ) -> Self {
        let diff_minutes =
            (local_time.unix_microseconds - server_time.unix_microseconds) as f64 / 60_000_000.0;

        Self {
            offset_in_minutes: (diff_minutes / 15.0).round() as i32 * 15,
        }
    }

    pub fn offset_in_minutes(&self) -> i32 {
        self.offset_in_minutes
    }

    pub fn offset_in_seconds(&self) -> i32 {
        self.offset_in_minutes * 60
    }

    /// The offset a time is really moved by. Falls back to `UTC+0` for an out-of-range value
    /// (real offsets never exceed ±14:00, so this only guards against garbage).
    pub(super) fn applied_offset_in_minutes(&self) -> i32 {
        if self.offset_in_minutes > -MINUTES_IN_ONE_DAY
            && self.offset_in_minutes < MINUTES_IN_ONE_DAY
        {
            self.offset_in_minutes
        } else {
            0
        }
    }

    /// `+01:00` / `-05:00`
    pub(super) fn push_offset_to_str(&self, dest: &mut String) {
        let offset_in_minutes = self.applied_offset_in_minutes();

        dest.push(if offset_in_minutes < 0 { '-' } else { '+' });

        let offset_in_minutes = offset_in_minutes.unsigned_abs();

        super::utils::push_zero_padded(dest, offset_in_minutes / 60, 2);
        dest.push(':');
        super::utils::push_zero_padded(dest, offset_in_minutes % 60, 2);
    }
}

impl std::fmt::Debug for TimeZone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut result = String::with_capacity(6);
        self.push_offset_to_str(&mut result);
        f.write_str(result.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(src: &str) -> DateTimeAsMicroseconds {
        DateTimeAsMicroseconds::parse_iso_string(src).unwrap()
    }

    #[test]
    fn from_server_and_local_time_rounds_to_15_minutes() {
        let server = utc("2021-04-25T17:30:03.000Z");

        let case = |offset_minutes: i64| {
            let mut local = server;
            local.add_minutes(offset_minutes);
            TimeZone::from_server_and_local_time(server, local).offset_in_minutes()
        };

        assert_eq!(60, case(60)); // exact +1h
        assert_eq!(60, case(58)); // +58m rounds up to 60
        assert_eq!(45, case(50)); // +50m -> nearest 15 -> 45
        assert_eq!(345, case(345)); // Nepal +5:45 stays exact
        assert_eq!(0, case(7)); // +7m rounds down to 0
        assert_eq!(-300, case(-300)); // -5h stays -300
    }

    #[test]
    fn offset_conversions() {
        let tz = TimeZone::from_minutes(90);
        assert_eq!(90, tz.offset_in_minutes());
        assert_eq!(5400, tz.offset_in_seconds());
    }

    #[test]
    fn debug_renders_the_offset() {
        assert_eq!("+00:00", format!("{:?}", TimeZone::utc()));
        assert_eq!("+01:00", format!("{:?}", TimeZone::from_minutes(60)));
        assert_eq!("-05:00", format!("{:?}", TimeZone::from_minutes(-300)));
        assert_eq!("+05:45", format!("{:?}", TimeZone::from_minutes(345)));
        assert_eq!("-00:30", format!("{:?}", TimeZone::from_minutes(-30)));
    }

    #[test]
    fn an_offset_of_a_day_or_more_falls_back_to_utc() {
        for minutes in [1440, -1440, i32::MAX, i32::MIN] {
            let tz = TimeZone::from_minutes(minutes);

            assert_eq!(0, tz.applied_offset_in_minutes(), "{}", minutes);
            assert_eq!("+00:00", format!("{:?}", tz), "{}", minutes);
        }

        assert_eq!(1439, TimeZone::from_minutes(1439).applied_offset_in_minutes());
        assert_eq!(-1439, TimeZone::from_minutes(-1439).applied_offset_in_minutes());
    }
}
