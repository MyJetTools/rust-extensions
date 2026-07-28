/// What the timer loop must do once `tick()` returned.
///
/// A tick which understood in the middle of its work that there is more to do -
/// but does not want to keep the current iteration running - returns
/// `Immediately`: it leaves the iteration and is started again straight away,
/// **with a fresh `iteration_timeout` window**. That is the way to do a long job
/// in portions without ever tripping the timer's per-iteration timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatTimerIteration {
    /// The iteration is complete - wait for the next scheduled tick as usual.
    WithInterval,
    /// Run `tick()` again right away, without waiting for the interval.
    Immediately,
}

impl RepeatTimerIteration {
    pub fn is_immediately(&self) -> bool {
        match self {
            RepeatTimerIteration::Immediately => true,
            RepeatTimerIteration::WithInterval => false,
        }
    }

    pub fn is_with_interval(&self) -> bool {
        !self.is_immediately()
    }
}

#[async_trait::async_trait]
pub trait MyTimerTick {
    async fn tick(&self) -> RepeatTimerIteration;
}
