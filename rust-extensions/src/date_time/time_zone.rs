use chrono::FixedOffset;

use super::DateTimeAsMicroseconds;

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

    /// chrono offset for this timezone. Falls back to `UTC+0` for an out-of-range value
    /// (real offsets never exceed ±14:00, so this only guards against garbage).
    pub fn to_fixed_offset(&self) -> FixedOffset {
        FixedOffset::east_opt(self.offset_in_seconds())
            .unwrap_or_else(|| FixedOffset::east_opt(0).unwrap())
    }
}

impl std::fmt::Debug for TimeZone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // FixedOffset's Display renders as `+01:00` / `-05:00`.
        write!(f, "{}", self.to_fixed_offset())
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
}
