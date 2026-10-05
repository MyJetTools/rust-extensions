use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use crate::{Logger, Startable};

use super::{
    timers_iteration::{execute_timer, execute_timers_iteration, get_timer_names, RegisteredTimer},
    MyTimerTick, RepeatTimerIteration,
};

pub struct MyTimer {
    interval: Duration,
    timers: Vec<RegisteredTimer>,
    iteration_timeout: Duration,
    delay_before_first_tick: bool,
    started: AtomicBool,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
}

impl MyTimer {
    pub fn new(interval: Duration, logger: Arc<dyn Logger + Send + Sync + 'static>) -> Self {
        Self {
            interval,
            timers: Vec::new(),
            iteration_timeout: Duration::from_secs(60),
            delay_before_first_tick: true,
            started: AtomicBool::new(false),
            logger,
        }
    }

    pub fn set_iteration_timeout(&mut self, iteration_timeout: Duration) {
        self.iteration_timeout = iteration_timeout;
    }

    pub fn new_with_execute_timeout(
        interval: Duration,
        iteration_timeout: Duration,
        logger: Arc<dyn Logger + Send + Sync + 'static>,
    ) -> Self {
        Self {
            interval,
            timers: Vec::new(),
            iteration_timeout,
            delay_before_first_tick: true,
            started: AtomicBool::new(false),
            logger,
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
        // The loop works with the ticks registered by `start()`: one registered
        // later would never be executed on schedule.
        if *self.started.get_mut() {
            panic!(
                "Timer [{}] with interval {:?} is already started: tick [{}] must be registered before start()",
                get_timer_names(&self.timers),
                self.interval,
                name
            );
        }

        for (timer_name, _) in &self.timers {
            if timer_name == name {
                panic!("Timer with the name [{}] is already registered", name);
            }
        }

        self.timers.push((name.to_string(), my_timer_tick));
    }

    /// Spawns the one and only loop of this timer. A second `start` panics -
    /// it would put a second loop on the very same ticks, and each of them
    /// would be executed twice per interval.
    pub fn start(&self) {
        // Claims the right to start before anything is spawned. `Relaxed` is
        // enough - what is needed is the atomicity of the swap, not an ordering
        // against anything else.
        if self
            .started
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            panic!(
                "Timer [{}] with interval {:?} is already started",
                get_timer_names(&self.timers),
                self.interval
            );
        }

        let timers = self.timers.clone();
        tokio::spawn(timer_loop(
            timers,
            self.interval,
            self.logger.clone(),
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

impl Startable for MyTimer {
    fn start(&self) {
        self.start();
    }
}

async fn timer_loop(
    timers: Vec<RegisteredTimer>,
    interval: Duration,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
    iteration_timeout: Duration,
    delay_before_first_tick: bool,
) {
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

    loop {
        let mut to_execute: Vec<&RegisteredTimer> = timers.iter().collect();

        loop {
            to_execute = execute_timers_iteration(&to_execute, &logger, iteration_timeout).await;

            // Ticks which left their iteration on purpose are restarted right
            // away - each with a fresh timeout window - and the interval is not
            // slept until every one of them is done.
            if to_execute.is_empty() {
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

    use crate::Logger;

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
                .try_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
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

            let mut timer = MyTimer::new(INTERVAL, Arc::new(TestLogger));
            timer.set_first_tick_before_delay();
            timer.register_timer("test", repeating_tick(&runs, 2));
            timer.start();

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

            let mut timer = MyTimer::new(INTERVAL, Arc::new(TestLogger));
            timer.set_first_tick_before_delay();
            timer.register_timer("repeating", repeating_tick(&repeating_runs, 2));
            timer.register_timer("calm", repeating_tick(&calm_runs, 0));
            timer.start();

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

            let mut timer = MyTimer::new(INTERVAL, Arc::new(TestLogger));
            timer.set_first_tick_before_delay();
            timer.register_timer(
                "panicking",
                Arc::new(PanickingTick { runs: runs.clone() }),
            );
            timer.start();

            wait_for(&runs, 1).await;

            // A panic answered nothing - it must not spin the loop.
            tokio::time::sleep(Duration::from_millis(200)).await;
            assert_eq!(runs.load(Ordering::SeqCst), 1);
        });
    }

    /// Sleeps far longer than the iteration timeout, then reports it got to the end.
    struct SlowTick {
        started: Arc<AtomicUsize>,
        finished: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl MyTimerTick for SlowTick {
        async fn tick(&self) -> RepeatTimerIteration {
            self.started.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(300)).await;
            self.finished.fetch_add(1, Ordering::SeqCst);
            RepeatTimerIteration::WithInterval
        }
    }

    /// With several ticks each one runs in its own task, and a task is not
    /// stopped by dropping its handle - a timed out tick has to be aborted.
    #[test]
    fn a_tick_which_times_out_is_cancelled_next_to_other_ticks() {
        rt().block_on(async {
            let started = Arc::new(AtomicUsize::new(0));
            let finished = Arc::new(AtomicUsize::new(0));
            let calm_runs = Arc::new(AtomicUsize::new(0));

            let mut timer = MyTimer::new_with_execute_timeout(
                INTERVAL,
                Duration::from_millis(50),
                Arc::new(TestLogger),
            );
            timer.set_first_tick_before_delay();
            timer.register_timer(
                "slow",
                Arc::new(SlowTick {
                    started: started.clone(),
                    finished: finished.clone(),
                }),
            );
            timer.register_timer("calm", repeating_tick(&calm_runs, 0));
            timer.start();

            wait_for(&started, 1).await;
            tokio::time::sleep(Duration::from_millis(500)).await;

            assert_eq!(calm_runs.load(Ordering::SeqCst), 1);
            assert_eq!(finished.load(Ordering::SeqCst), 0);
        });
    }

    #[test]
    fn second_start_panics_and_does_not_spawn_a_second_loop() {
        rt().block_on(async {
            let runs = Arc::new(AtomicUsize::new(0));

            let mut timer = MyTimer::new(INTERVAL, Arc::new(TestLogger));
            timer.set_first_tick_before_delay();
            timer.register_timer("first", repeating_tick(&runs, 0));
            timer.register_timer("second", repeating_tick(&runs, 0));
            timer.start();

            let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                timer.start();
            }));

            let panic = panicked.expect_err("The second start must panic");
            assert_eq!(
                panic.downcast_ref::<String>().unwrap(),
                "Timer [first, second] with interval 30s is already started"
            );

            // The loop of the first start is alive, and it is the only one: a
            // second loop would have made the first tick of its own by now.
            wait_for(&runs, 2).await;
            tokio::time::sleep(Duration::from_millis(200)).await;
            assert_eq!(runs.load(Ordering::SeqCst), 2);
        });
    }
}
