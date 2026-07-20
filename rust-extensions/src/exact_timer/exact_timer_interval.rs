use std::time::Duration;

/// Interval at which a [`MyExactTimer`](super::MyExactTimer) is triggered.
///
/// Every variant evenly divides a minute (for sub-minute intervals) or an hour
/// (for minute intervals). Because the Unix epoch is itself aligned to the
/// minute and the hour, aligning a tick to the epoch makes it fire exactly on
/// the natural wall-clock marks - e.g. `Every5Seconds` fires at seconds
/// `:00, :05, :10, ... :55` and `Every5Minutes` at minutes `:00, :05, ... :55`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactTimerInterval {
    Every1Second,
    Every5Seconds,
    Every10Seconds,
    Every15Seconds,
    Every20Seconds,
    Every30Seconds,
    Every1Minute,
    Every5Minutes,
    Every10Minutes,
    Every15Minutes,
    Every20Minutes,
    Every30Minutes,
}

impl ExactTimerInterval {
    pub fn get_duration(&self) -> Duration {
        match self {
            ExactTimerInterval::Every1Second => Duration::from_secs(1),
            ExactTimerInterval::Every5Seconds => Duration::from_secs(5),
            ExactTimerInterval::Every10Seconds => Duration::from_secs(10),
            ExactTimerInterval::Every15Seconds => Duration::from_secs(15),
            ExactTimerInterval::Every20Seconds => Duration::from_secs(20),
            ExactTimerInterval::Every30Seconds => Duration::from_secs(30),
            ExactTimerInterval::Every1Minute => Duration::from_secs(60),
            ExactTimerInterval::Every5Minutes => Duration::from_secs(5 * 60),
            ExactTimerInterval::Every10Minutes => Duration::from_secs(10 * 60),
            ExactTimerInterval::Every15Minutes => Duration::from_secs(15 * 60),
            ExactTimerInterval::Every20Minutes => Duration::from_secs(20 * 60),
            ExactTimerInterval::Every30Minutes => Duration::from_secs(30 * 60),
        }
    }

    pub fn get_duration_micros(&self) -> u64 {
        self.get_duration().as_micros() as u64
    }
}
