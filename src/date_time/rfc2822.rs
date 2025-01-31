use super::{DateTimeStruct, MONTHS};

impl DateTimeStruct {
    pub fn to_rfc2822(&self) -> String {
        let mut result = String::with_capacity(32);

        result.push_str(self.get_day_of_week_as_str());
        result.push_str(", ");

        if self.day < 10 {
            result.push('0');
        }

        result.push_str(self.day.to_string().as_str());
        result.push(' ');

        result.push_str(MONTHS[self.month as usize - 1]);
        result.push(' ');

        result.push_str(self.year.to_string().as_str());

        result.push(' ');
        self.time.push_time_no_micros_to_str(&mut result);

        result.push_str(" +0000");

        result
    }
}

#[cfg(test)]
mod test {

    use crate::date_time::DateTimeAsMicroseconds;

    #[test]
    fn test_2822() {
        let src = DateTimeAsMicroseconds::new(1704379539839324);

        assert_eq!(src.to_rfc2822(), "Thu, 04 Jan 2024 14:45:39 +0000");
    }
}
