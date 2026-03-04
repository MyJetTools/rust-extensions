use super::{DateTimeStruct, TimeStruct, MONTHS, WEEKS};

fn is_month(src: &str) -> Option<u32> {
    let mut result = 1;
    for month in MONTHS {
        if month == src {
            return Some(result);
        }

        result += 1;
    }
    None
}

impl DateTimeStruct {
    pub fn to_rfc5322(&self) -> String {
        let mut result = String::with_capacity(24);

        result.push_str(MONTHS[self.month as usize - 1]);
        result.push(' ');

        if self.day < 10 {
            result.push(' ');
        }

        result.push_str(self.day.to_string().as_str());
        result.push(' ');
        self.time.push_time_no_micros_to_str(&mut result);

        result.push(' ');

        result.push_str(self.year.to_string().as_str());

        result.push_str(" GMT");

        result
    }
    pub fn parse_rfc_5322(src: &str) -> Option<Self> {
        let mut no = 0;

        let mut year = 0;
        let mut month = 0;
        let mut day = 0;
        let mut time = None;

        for itm in src.split(' ') {
            if itm == "" {
                continue;
            }

            match no {
                0 => {
                    if let Some(found_month) = is_month(itm) {
                        month = found_month;
                    } else if WEEKS.contains(&itm) {
                        continue;
                    } else {
                        return None;
                    }
                }

                1 => {
                    day = match itm.parse() {
                        Ok(result) => result,
                        Err(_) => return None,
                    }
                }

                2 => {
                    time = TimeStruct::parse_from_str(itm);
                }

                3 => {
                    year = match itm.parse() {
                        Ok(result) => result,
                        Err(_) => return None,
                    }
                }

                _ => {
                    break;
                }
            }

            no += 1;
        }

        DateTimeStruct {
            year,
            month,
            day,
            time: time?,
            dow: None,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use crate::date_time::DateTimeStruct;

    #[test]
    fn test() {
        let result = DateTimeStruct::parse_rfc_5322("Sep  8 18:41:54 2032 GMT").unwrap();

        assert_eq!(2032, result.year);
        assert_eq!(9, result.month);
        assert_eq!(8, result.day);

        assert_eq!(18, result.time.hour);
        assert_eq!(41, result.time.min);
        assert_eq!(54, result.time.sec);

        assert_eq!(0, result.time.micros);
    }

    #[test]
    fn test_parse_forward_and_back() {
        let result = DateTimeStruct::parse_rfc_5322("Sep  8 18:41:54 2032 GMT").unwrap();

        assert_eq!("Sep  8 18:41:54 2032 GMT", result.to_rfc5322())
    }
}
