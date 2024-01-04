use super::DateTimeStruct;

impl DateTimeStruct {
    pub fn to_compact_date_time_string(&self) -> String {
        format!(
            "{:04}{:02}{:02}{:02}{:02}{:02}",
            self.year, self.month, self.day, self.time.hour, self.time.min, self.time.sec
        )
    }
    pub fn parse_compact_date_time(src: &[u8]) -> Option<Self> {
        let year = super::utils::parse_four_digits(&src[0..4])?;

        let month = super::utils::parse_two_digits(&src[4..6])?;

        let day = super::utils::parse_two_digits(&src[6..8])?;

        let hour = super::utils::parse_two_digits(&src[8..10])?;

        let min = super::utils::parse_two_digits(&src[10..12])?;

        let sec = super::utils::parse_two_digits(&src[12..14])?;

        Some(DateTimeStruct {
            year,
            month,
            day,
            time: super::TimeStruct {
                hour,
                min,
                sec,
                micros: 0,
            },
            dow: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::date_time::DateTimeStruct;

    #[test]
    pub fn test_parse_compact_string() {
        let src = "20210425173043";
        let dt = DateTimeStruct::parse_compact_date_time(src.as_bytes()).unwrap();

        assert_eq!(dt.year, 2021);
        assert_eq!(dt.month, 04);
        assert_eq!(dt.day, 25);
        assert_eq!(dt.time.hour, 17);
        assert_eq!(dt.time.min, 30);
        assert_eq!(dt.time.sec, 43);
        assert_eq!(dt.time.micros, 0);
    }
}
