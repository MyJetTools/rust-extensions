use crate::date_time::{AtomicDateTimeAsMicroseconds, DateTimeAsMicroseconds, DateTimeDuration};

pub struct AtomicStopWatch {
    start_time: AtomicDateTimeAsMicroseconds,
    stop_time: AtomicDateTimeAsMicroseconds,
}

//Bug - negative duration;
impl AtomicStopWatch {
    pub fn new() -> Self {
        let now = DateTimeAsMicroseconds::now();
        let start_time: AtomicDateTimeAsMicroseconds = now.clone().into();
        let stop_time: AtomicDateTimeAsMicroseconds = now.into();

        Self {
            start_time,
            stop_time,
        }
    }

    pub fn reset_and_start(&self) {
        let now = DateTimeAsMicroseconds::now();
        self.start_time.update(now);
        self.stop_time.update(now);
    }

    pub fn pause(&self) {
        let now = DateTimeAsMicroseconds::now();
        self.stop_time.update(now);
    }

    pub fn duration(&self) -> DateTimeDuration {
        self.stop_time
            .duration_since(self.start_time.as_date_time())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_duration() {
        let sw = AtomicStopWatch::new();

        sw.reset_and_start();

        std::thread::sleep(std::time::Duration::from_millis(10));
        sw.pause();

        println!("{:?}", sw.duration().to_string());
    }
}
