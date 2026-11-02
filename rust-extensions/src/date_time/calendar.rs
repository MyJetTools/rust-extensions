// The proleptic Gregorian calendar, counted in days from 1970-01-01. The two conversions are
// Howard Hinnant's `days_from_civil` and `civil_from_days`:
// https://howardhinnant.github.io/date_algorithms.html
//
// Both count a year from March, so the leap day is the last day of a year, and split the time
// into eras of 400 years, after which the calendar repeats.

const DAYS_IN_ONE_ERA: i64 = 146_097;

/// 0000-03-01, the start of an era, is this many days before 1970-01-01.
const DAYS_TO_UNIX_EPOCH: i64 = 719_468;

/// A date inside these years always fits into `i64` microseconds.
const MIN_YEAR: i32 = -262_143;
const MAX_YEAR: i32 = 262_142;

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

pub(super) fn is_valid_date(year: i32, month: u32, day: u32) -> bool {
    year >= MIN_YEAR && year <= MAX_YEAR && day >= 1 && day <= days_in_month(year, month)
}

/// Days from 1970-01-01 to a date which passed [`is_valid_date`].
pub(super) fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = if month <= 2 {
        year as i64 - 1
    } else {
        year as i64
    };

    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let month_from_march = ((month + 9) % 12) as i64;
    let day_of_year = (153 * month_from_march + 2) / 5 + day as i64 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

    era * DAYS_IN_ONE_ERA + day_of_era - DAYS_TO_UNIX_EPOCH
}

/// The `(year, month, day)` a number of days from 1970-01-01 lands on.
pub(super) fn civil_from_days(unix_days: i64) -> (i32, u32, u32) {
    let days = unix_days + DAYS_TO_UNIX_EPOCH;

    let era = days.div_euclid(DAYS_IN_ONE_ERA);
    let day_of_era = days.rem_euclid(DAYS_IN_ONE_ERA);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;

    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };

    let year = year_of_era + era * 400 + if month <= 2 { 1 } else { 0 };

    (year as i32, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_unix_epoch_is_day_zero() {
        assert_eq!(0, days_from_civil(1970, 1, 1));
        assert_eq!((1970, 1, 1), civil_from_days(0));
    }

    #[test]
    fn days_before_the_epoch_are_negative() {
        assert_eq!(-1, days_from_civil(1969, 12, 31));
        assert_eq!((1969, 12, 31), civil_from_days(-1));
        assert_eq!((1, 1, 1), civil_from_days(days_from_civil(1, 1, 1)));
        assert_eq!((-1, 3, 1), civil_from_days(days_from_civil(-1, 3, 1)));
    }

    #[test]
    fn a_century_is_a_leap_year_only_each_400_years() {
        assert!(is_valid_date(2000, 2, 29));
        assert!(is_valid_date(2024, 2, 29));
        assert!(!is_valid_date(1900, 2, 29));
        assert!(!is_valid_date(2023, 2, 29));
    }

    #[test]
    fn invalid_dates() {
        for (year, month, day) in [
            (2021, 0, 1),
            (2021, 13, 1),
            (2021, 1, 0),
            (2021, 1, 32),
            (2021, 4, 31),
            (MIN_YEAR - 1, 1, 1),
            (MAX_YEAR + 1, 1, 1),
        ] {
            assert!(!is_valid_date(year, month, day), "{year}-{month}-{day}");
        }
    }

    /// Every day of 800 years, so both eras around the epoch are walked through.
    #[test]
    fn days_round_trip() {
        let from = days_from_civil(1600, 1, 1);
        let to = days_from_civil(2400, 1, 1);

        for unix_days in from..to {
            let (year, month, day) = civil_from_days(unix_days);
            assert!(is_valid_date(year, month, day));
            assert_eq!(unix_days, days_from_civil(year, month, day));
        }
    }
}
