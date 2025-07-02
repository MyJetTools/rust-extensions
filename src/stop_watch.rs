use std::time::{Duration, SystemTime};

use crate::date_time::DateTimeAsMicroseconds;

use super::duration_utils::duration_to_string;

pub struct StopWatch {
    start_time: SystemTime,
}

//Bug - negative duration;
impl StopWatch {
    pub fn new() -> Self {
        let now = SystemTime::now();
        Self { start_time: now }
    }

    pub fn reset(&mut self) {
        self.start_time = SystemTime::now();
    }

    #[deprecated(note = "No need to use this function")]
    pub fn start(&mut self) {}

    #[deprecated(note = "No need to use this function")]
    pub fn pause(&mut self) {}

    pub fn duration(&self) -> Duration {
        let now = SystemTime::now();
        now.duration_since(self.start_time).unwrap()
    }

    pub fn duration_as_string(&self) -> String {
        let duration = self.duration();
        duration_to_string(duration)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_negative_duration() {
        let mut sw = StopWatch::new();

        sw.start();
        sw.pause();

        println!("{:?}", sw.duration_as_string());
    }
}
