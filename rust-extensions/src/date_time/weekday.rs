/// A day of the week. The variants count from Monday, which is what
/// [`super::TimeStruct::to_micro_second_withing_week`] relies on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Weekday {
    /// The day of the week of a date, given in days from 1970-01-01 - a Thursday.
    pub(super) fn from_unix_days(unix_days: i64) -> Self {
        match (unix_days + 3).rem_euclid(7) {
            0 => Weekday::Mon,
            1 => Weekday::Tue,
            2 => Weekday::Wed,
            3 => Weekday::Thu,
            4 => Weekday::Fri,
            5 => Weekday::Sat,
            _ => Weekday::Sun,
        }
    }

    pub fn num_days_from_monday(&self) -> u32 {
        *self as u32
    }

    pub fn num_days_from_sunday(&self) -> u32 {
        (*self as u32 + 1) % 7
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_unix_days() {
        assert_eq!(Weekday::Thu, Weekday::from_unix_days(0));
        assert_eq!(Weekday::Sun, Weekday::from_unix_days(3));
        assert_eq!(Weekday::Mon, Weekday::from_unix_days(4));
        assert_eq!(Weekday::Wed, Weekday::from_unix_days(-1));
        assert_eq!(Weekday::Thu, Weekday::from_unix_days(-7));
    }

    #[test]
    fn num_days() {
        assert_eq!(0, Weekday::Mon.num_days_from_monday());
        assert_eq!(6, Weekday::Sun.num_days_from_monday());

        assert_eq!(0, Weekday::Sun.num_days_from_sunday());
        assert_eq!(1, Weekday::Mon.num_days_from_sunday());
        assert_eq!(6, Weekday::Sat.num_days_from_sunday());
    }
}
