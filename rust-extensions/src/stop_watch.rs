use std::time::{Duration, SystemTime};

use crate::date_time::DateTimeAsMicroseconds;

use super::duration_utils::duration_to_string;

pub struct StopWatch(SystemTime);

impl StopWatch {
    pub fn new() -> Self {
        let now = SystemTime::now();
        Self(now)
    }

    pub fn reset(&mut self) {
        self.0 = SystemTime::now();
    }

    #[deprecated(note = "No need to use this function")]
    pub fn start(&mut self) {}

    #[deprecated(note = "No need to use this function")]
    pub fn pause(&mut self) {}

    /// Time since `new` / `reset`. It is the wall clock, so if the clock is moved
    /// back past the start this is zero rather than a panic.
    pub fn duration(&self) -> Duration {
        let now = SystemTime::now();
        now.duration_since(self.0).unwrap_or(Duration::ZERO)
    }

    pub fn duration_as_string(&self) -> String {
        let duration = self.duration();
        duration_to_string(duration)
    }

    pub fn get_start_time(&self) -> DateTimeAsMicroseconds {
        self.0.into()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_negative_duration() {
        let sw = StopWatch::new();

        println!("{:?}", sw.duration_as_string());
    }

    #[test]
    fn test_reset_starts_over() {
        let mut sw = StopWatch::new();
        sw.0 = SystemTime::now() - Duration::from_secs(60);

        sw.reset();

        assert!(sw.duration() < Duration::from_secs(60));
    }

    #[test]
    fn test_a_start_in_the_future_is_zero() {
        let mut sw = StopWatch::new();
        sw.0 = SystemTime::now() + Duration::from_secs(60);

        assert_eq!(sw.duration(), Duration::ZERO);
    }
}
