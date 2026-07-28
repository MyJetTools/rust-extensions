use std::{sync::Arc, time::Duration};

use crate::{ApplicationStates, Logger};

use super::{
    timers_iteration::{execute_timer, execute_timers_iteration, RegisteredTimer},
    MyTimerTick, RepeatTimerIteration,
};

pub struct MyTimer {
    interval: Duration,
    timers: Vec<RegisteredTimer>,
    iteration_timeout: Duration,
    delay_before_first_tick: bool,
}

impl MyTimer {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            timers: Vec::new(),
            iteration_timeout: Duration::from_secs(60),
            delay_before_first_tick: true,
        }
    }

    pub fn set_iteration_timeout(&mut self, iteration_timeout: Duration) {
        self.iteration_timeout = iteration_timeout;
    }

    pub fn new_with_execute_timeout(interval: Duration, iteration_timeout: Duration) -> Self {
        Self {
            interval,
            timers: Vec::new(),
            iteration_timeout,
            delay_before_first_tick: true,
        }
    }

    pub fn set_first_tick_before_delay(&mut self) {
        self.delay_before_first_tick = false;
    }

    pub fn register_timer(
        &mut self,
        name: &str,
        my_timer_tick: Arc<dyn MyTimerTick + Send + Sync + 'static>,
    ) {
        for (timer_name, _) in &self.timers {
            if timer_name == name {
                panic!("Timer with the name [{}] is already registered", name);
            }
        }

        self.timers.push((name.to_string(), my_timer_tick));
    }

    pub fn start(
        &self,
        app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
        logger: Arc<dyn Logger + Send + Sync + 'static>,
    ) {
        let timers = self.timers.clone();
        tokio::spawn(timer_loop(
            timers,
            self.interval,
            app_states,
            logger,
            self.iteration_timeout,
            self.delay_before_first_tick,
        ));
    }

    /// Executes the named tick once, out of schedule. There is no interval to
    /// wait for here, so a `RepeatTimerIteration::Immediately` is handed back to
    /// the caller rather than acted upon.
    pub async fn execute_timer(&self, timer_name: &str) -> RepeatTimerIteration {
        for (timer_id, timer_tick) in &self.timers {
            if timer_id == timer_name {
                return tokio::spawn(execute_timer(timer_tick.clone()))
                    .await
                    .unwrap();
            }
        }

        panic!("Timer with the name [{}] is not found", timer_name);
    }
}

async fn timer_loop(
    timers: Vec<RegisteredTimer>,
    interval: Duration,
    app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
    iteration_timeout: Duration,
    delay_before_first_tick: bool,
) {
    while !app_states.is_initialized() {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    for (timer_id, _) in &timers {
        let message = format!(
            "Timer {} is started with delay {} sec",
            timer_id,
            interval.as_secs()
        );

        logger.write_info(timer_id.to_string().into(), message.into(), None.into());
    }

    if delay_before_first_tick {
        tokio::time::sleep(interval).await;
    }

    while !app_states.is_shutting_down() {
        let mut to_execute: Vec<&RegisteredTimer> = timers.iter().collect();

        loop {
            to_execute = execute_timers_iteration(&to_execute, &logger, iteration_timeout).await;

            // Ticks which left their iteration on purpose are restarted right
            // away - each with a fresh timeout window - and the interval is not
            // slept until every one of them is done.
            if to_execute.is_empty() || app_states.is_shutting_down() {
                break;
            }
        }

        tokio::time::sleep(interval).await;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use crate::{ApplicationStates, Logger};

    use super::{MyTimer, MyTimerTick, RepeatTimerIteration};

    /// Far longer than any test waits for - so anything that happens within a
    /// test window happened because a tick asked for it, not because the
    /// interval elapsed.
    const INTERVAL: Duration = Duration::from_secs(30);

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
    }

    struct TestLogger;

    impl Logger for TestLogger {
        fn write_info(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
        fn write_warning(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
        fn write_error(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
        fn write_fatal_error(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
        fn write_debug_info(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
    }

    struct TestAppStates;

    impl ApplicationStates for TestAppStates {
        fn is_initialized(&self) -> bool {
            true
        }
        fn is_shutting_down(&self) -> bool {
            false
        }
    }

    /// Asks for `immediate_repeats` extra passes - as a tick which leaves the
    /// iteration early to reset its timeout would - and then settles down.
    struct RepeatingTick {
        runs: Arc<AtomicUsize>,
        immediate_repeats: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl MyTimerTick for RepeatingTick {
        async fn tick(&self) -> RepeatTimerIteration {
            self.runs.fetch_add(1, Ordering::SeqCst);

            if self
                .immediate_repeats
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                    if left == 0 {
                        None
                    } else {
                        Some(left - 1)
                    }
                })
                .is_ok()
            {
                return RepeatTimerIteration::Immediately;
            }

            RepeatTimerIteration::WithInterval
        }
    }

    struct PanickingTick {
        runs: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl MyTimerTick for PanickingTick {
        async fn tick(&self) -> RepeatTimerIteration {
            self.runs.fetch_add(1, Ordering::SeqCst);
            panic!("tick is panicking on purpose");
        }
    }

    fn repeating_tick(runs: &Arc<AtomicUsize>, immediate_repeats: usize) -> Arc<RepeatingTick> {
        Arc::new(RepeatingTick {
            runs: runs.clone(),
            immediate_repeats: AtomicUsize::new(immediate_repeats),
        })
    }

    async fn wait_for(runs: &Arc<AtomicUsize>, expected: usize) {
        for _ in 0..400 {
            if runs.load(Ordering::SeqCst) >= expected {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("Expected {} runs, got {}", expected, runs.load(Ordering::SeqCst));
    }

    #[test]
    fn immediately_runs_again_without_waiting_for_the_interval() {
        rt().block_on(async {
            let runs = Arc::new(AtomicUsize::new(0));

            let mut timer = MyTimer::new(INTERVAL);
            timer.set_first_tick_before_delay();
            timer.register_timer("test", repeating_tick(&runs, 2));
            timer.start(Arc::new(TestAppStates), Arc::new(TestLogger));

            // 1 scheduled tick + 2 immediate repeats, all well inside INTERVAL.
            wait_for(&runs, 3).await;

            // And then it settles: the 4th pass waits for the interval.
            tokio::time::sleep(Duration::from_millis(200)).await;
            assert_eq!(runs.load(Ordering::SeqCst), 3);
        });
    }

    #[test]
    fn only_the_tick_which_asked_is_repeated() {
        rt().block_on(async {
            let repeating_runs = Arc::new(AtomicUsize::new(0));
            let calm_runs = Arc::new(AtomicUsize::new(0));

            let mut timer = MyTimer::new(INTERVAL);
            timer.set_first_tick_before_delay();
            timer.register_timer("repeating", repeating_tick(&repeating_runs, 2));
            timer.register_timer("calm", repeating_tick(&calm_runs, 0));
            timer.start(Arc::new(TestAppStates), Arc::new(TestLogger));

            wait_for(&repeating_runs, 3).await;
            tokio::time::sleep(Duration::from_millis(200)).await;

            // The neighbour keeps its own schedule - it is not dragged into the
            // extra passes.
            assert_eq!(repeating_runs.load(Ordering::SeqCst), 3);
            assert_eq!(calm_runs.load(Ordering::SeqCst), 1);
        });
    }

    #[test]
    fn panicked_tick_is_not_repeated() {
        rt().block_on(async {
            let runs = Arc::new(AtomicUsize::new(0));

            let mut timer = MyTimer::new(INTERVAL);
            timer.set_first_tick_before_delay();
            timer.register_timer(
                "panicking",
                Arc::new(PanickingTick { runs: runs.clone() }),
            );
            timer.start(Arc::new(TestAppStates), Arc::new(TestLogger));

            wait_for(&runs, 1).await;

            // A panic answered nothing - it must not spin the loop.
            tokio::time::sleep(Duration::from_millis(200)).await;
            assert_eq!(runs.load(Ordering::SeqCst), 1);
        });
    }
}
