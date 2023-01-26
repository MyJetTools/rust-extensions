use crate::date_time::DateTimeAsMicroseconds;

use super::DateTimeDuration;

pub struct ClientServerTimeDifference {
    minutes: f64,
}

impl ClientServerTimeDifference {
    pub fn new(client_time: DateTimeAsMicroseconds, server_time: DateTimeAsMicroseconds) -> Self {
        let duration = client_time.duration_since(server_time);

        match duration {
            DateTimeDuration::Positive(duration) => {
                let minutes = (duration.as_secs() / 60) as f64;
                Self { minutes }
            }
            DateTimeDuration::Negative(duration) => {
                let minutes = (duration.as_secs() / 60) as f64;
                Self { minutes: -minutes }
            }
            DateTimeDuration::Zero => Self { minutes: 0.0 },
        }
    }

    pub fn difference_in_hours(&self) -> i64 {
        let result = self.minutes / 60.0;
        result.round() as i64
    }

    pub fn difference_in_half_hours(&self) -> i64 {
        let result = self.minutes / 30.0;
        result.round() as i64
    }
}

#[cfg(test)]
mod test {
    use crate::date_time::DateTimeAsMicroseconds;

    #[test]
    fn test_difference_in_hours() {
        let client_time = DateTimeAsMicroseconds::from_str("2021-04-25T13:00:00").unwrap();

        let difference = client_time.get_client_server_time_difference(
            DateTimeAsMicroseconds::from_str("2021-04-25T13:00:00").unwrap(),
        );

        assert_eq!(0, difference.difference_in_hours());

        let difference = client_time.get_client_server_time_difference(
            DateTimeAsMicroseconds::from_str("2021-04-25T13:10:00").unwrap(),
        );

        assert_eq!(0, difference.difference_in_hours());

        let difference = client_time.get_client_server_time_difference(
            DateTimeAsMicroseconds::from_str("2021-04-25T13:50:00").unwrap(),
        );

        assert_eq!(1, difference.difference_in_hours());
    }

    #[test]
    fn test_difference_in_half_hours() {
        let client_time = DateTimeAsMicroseconds::from_str("2021-04-25T13:00:00").unwrap();

        let difference = client_time.get_client_server_time_difference(
            DateTimeAsMicroseconds::from_str("2021-04-25T13:00:00").unwrap(),
        );

        assert_eq!(0, difference.difference_in_half_hours());

        let difference = client_time.get_client_server_time_difference(
            DateTimeAsMicroseconds::from_str("2021-04-25T13:25:00").unwrap(),
        );

        assert_eq!(1, difference.difference_in_half_hours());

        let difference = client_time.get_client_server_time_difference(
            DateTimeAsMicroseconds::from_str("2021-04-25T13:35:00").unwrap(),
        );

        assert_eq!(1, difference.difference_in_half_hours());

        let difference = client_time.get_client_server_time_difference(
            DateTimeAsMicroseconds::from_str("2021-04-25T13:55:00").unwrap(),
        );

        assert_eq!(2, difference.difference_in_half_hours());

        let difference = client_time.get_client_server_time_difference(
            DateTimeAsMicroseconds::from_str("2021-04-25T14:04:00").unwrap(),
        );

        assert_eq!(2, difference.difference_in_half_hours());
    }
}
