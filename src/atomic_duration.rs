use std::{sync::atomic::AtomicU64, time::Duration};

pub struct AtomicDuration {
    micros: AtomicU64,
}

impl AtomicDuration {
    pub fn from_micros(micros: u64) -> Self {
        Self {
            micros: AtomicU64::new(micros),
        }
    }

    pub fn from_millis(micros: u64) -> Self {
        Self {
            micros: AtomicU64::new(micros * 1000),
        }
    }
    pub fn from_secs(secs: u64) -> Self {
        Self {
            micros: AtomicU64::new(secs * 1000_000),
        }
    }

    pub fn get_micros(&self) -> u64 {
        self.micros.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn to_duration(&self) -> Duration {
        let micros = self.get_micros();
        Duration::from_micros(micros)
    }
}

impl Into<AtomicDuration> for Duration {
    fn into(self) -> AtomicDuration {
        let micros = self.as_micros() as u64;
        AtomicDuration {
            micros: AtomicU64::new(micros),
        }
    }
}

impl Into<Duration> for &'_ AtomicDuration {
    fn into(self) -> Duration {
        self.to_duration()
    }
}

impl Clone for AtomicDuration {
    fn clone(&self) -> Self {
        let micros = self.get_micros();
        Self {
            micros: AtomicU64::new(micros),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::AtomicDuration;

    #[test]
    fn test_duration() {
        let duration = Duration::from_secs(1);

        let atomic: AtomicDuration = duration.into();

        let result = atomic.to_duration();

        assert_eq!(result.as_secs(), 1)
    }
}
