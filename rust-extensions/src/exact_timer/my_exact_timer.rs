use std::{collections::HashMap, panic::AssertUnwindSafe, sync::Arc, time::Duration};

use futures::FutureExt;

use crate::{ApplicationStates, Logger, MyTimerTick};

use super::ExactTimerInterval;

/// A timer that triggers exactly on the wall-clock marks of the given
/// [`ExactTimerInterval`] (e.g. `Every5Seconds` fires at seconds `:00, :05,
/// :10, ...`), unlike [`MyTimer`](crate::MyTimer) which merely sleeps
/// `interval` between ticks and therefore drifts.
///
/// Ticks reuse the same [`MyTimerTick`] trait as [`MyTimer`](crate::MyTimer),
/// so an existing tick implementation can be registered on either timer.
///
/// After a tick finishes, the next fire time is recomputed from the moment the
/// tick completed, so a slow tick simply skips to the next aligned mark instead
/// of accumulating drift.
pub struct MyExactTimer {
    interval: ExactTimerInterval,
    timers: Vec<(String, Arc<dyn MyTimerTick + Send + Sync + 'static>)>,
    iteration_timeout: Duration,
}

impl MyExactTimer {
    pub fn new(interval: ExactTimerInterval) -> Self {
        Self {
            interval,
            timers: Vec::new(),
            iteration_timeout: Duration::from_secs(60),
        }
    }

    pub fn new_with_execute_timeout(
        interval: ExactTimerInterval,
        iteration_timeout: Duration,
    ) -> Self {
        Self {
            interval,
            timers: Vec::new(),
            iteration_timeout,
        }
    }

    pub fn set_iteration_timeout(&mut self, iteration_timeout: Duration) {
        self.iteration_timeout = iteration_timeout;
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
        tokio::spawn(exact_timer_loop(
            timers,
            self.interval,
            app_states,
            logger,
            self.iteration_timeout,
        ));
    }

    pub async fn execute_timer(&self, timer_name: &str) {
        for (timer_id, timer_tick) in &self.timers {
            if timer_id == timer_name {
                tokio::spawn(execute_timer(timer_tick.clone()))
                    .await
                    .unwrap();
                return;
            }
        }

        panic!("Timer with the name [{}] is not found", timer_name);
    }
}

async fn exact_timer_loop(
    timers: Vec<(String, Arc<dyn MyTimerTick + Send + Sync + 'static>)>,
    interval: ExactTimerInterval,
    app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
    iteration_timeout: Duration,
) {
    let interval_micros = interval.get_duration_micros();

    while !app_states.is_initialized() {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    for (timer_id, _) in &timers {
        let message = format!("Exact timer {} is started with interval {:?}", timer_id, interval);

        logger.write_info(timer_id.to_string().into(), message.into(), None.into());
    }

    while !app_states.is_shutting_down() {
        // Based on the moment we finished the previous iteration, compute the
        // next aligned mark and precisely sleep up to it.
        sleep_till_next_tick(interval_micros, app_states.as_ref()).await;

        if app_states.is_shutting_down() {
            break;
        }

        if timers.len() == 1 {
            let (timer_id, timer) = &timers[0];
            let tick_future = AssertUnwindSafe(execute_timer(timer.clone())).catch_unwind();

            match tokio::time::timeout(iteration_timeout, tick_future).await {
                Ok(Ok(_)) => {}
                Ok(Err(_panic)) => {
                    let message = format!("Timer {} is panicked", timer_id);
                    println!("{}", message);
                    logger.write_error(timer_id.to_string().into(), message.into(), None.into());
                }
                Err(err) => {
                    println!("Timer {} is time outed with err: {:?}", timer_id, err);
                }
            }
        } else {
            let mut timer_handles = HashMap::new();
            for (timer_id, timer) in &timers {
                let handle = tokio::spawn(execute_timer(timer.clone()));
                timer_handles.insert(timer_id, handle);
            }

            for (timer_id, timer_handler) in timer_handles {
                match tokio::time::timeout(iteration_timeout, timer_handler).await {
                    Ok(result) => {
                        if let Err(err) = result {
                            let message = format!("Timer {} is panicked. Err: {:?}", timer_id, err);
                            let timer_id = timer_id.to_string();
                            let logger = logger.clone();

                            tokio::spawn(async move {
                                println!("{}", message);
                                logger.write_error(timer_id.into(), message.into(), None.into());
                            });
                        }
                    }
                    Err(err) => {
                        println!("Timer {} is time outed with err: {:?}", timer_id, err);
                    }
                }
            }
        }
    }
}

async fn execute_timer(timer: Arc<dyn MyTimerTick + Send + Sync + 'static>) {
    timer.tick().await;
}

/// Sleeps until the next wall-clock mark aligned to `interval_micros`.
///
/// The target mark is computed once, from the current time, then approached
/// with a coarse-to-fine ladder that re-measures the remaining time on every
/// iteration. This keeps the loop responsive to shutdown (a long interval never
/// blocks for more than 10 seconds); once under one second remains, a single
/// exact sleep lands precisely on the mark. A backward wall-clock jump is
/// detected and the target re-aligned, so the total wait never exceeds one
/// interval.
async fn sleep_till_next_tick(
    interval_micros: u64,
    app_states: &(dyn ApplicationStates + Send + Sync + 'static),
) {
    let mut target_micros = get_next_tick_micros(get_now_micros(), interval_micros);

    loop {
        if app_states.is_shutting_down() {
            return;
        }

        let now_micros = get_now_micros();

        if now_micros >= target_micros {
            return;
        }

        // Guard against a backward wall-clock jump (NTP / suspend-resume): the
        // fixed target must never be more than one interval away, otherwise the
        // timer would go silent for the whole magnitude of the jump. Re-align to
        // the next mark from the current time when that happens.
        target_micros = realign_target_on_clock_jump(target_micros, now_micros, interval_micros);

        let remaining = Duration::from_micros(target_micros - now_micros);

        tokio::time::sleep(get_sleep_chunk(remaining)).await;
    }
}

/// Keeps the wait bounded to one interval. In steady state the target stays
/// fixed (so the mark is hit precisely). Only a backward wall-clock jump can
/// push it more than one interval into the future, and then we re-derive the
/// next aligned mark from `now_micros`.
fn realign_target_on_clock_jump(target_micros: u64, now_micros: u64, interval_micros: u64) -> u64 {
    if target_micros.saturating_sub(now_micros) > interval_micros {
        get_next_tick_micros(now_micros, interval_micros)
    } else {
        target_micros
    }
}

/// The next multiple of `interval_micros` that is strictly greater than
/// `now_micros`. Aligned to the Unix epoch, which - since the epoch sits on a
/// minute/hour boundary and every interval evenly divides a minute or an hour -
/// lands on the natural wall-clock marks.
fn get_next_tick_micros(now_micros: u64, interval_micros: u64) -> u64 {
    (now_micros / interval_micros + 1) * interval_micros
}

/// Coarse-to-fine sleep chunk: sleep big while far from the mark, stepping down
/// as it approaches. Once under one second, sleep the exact remainder in a
/// single precise sleep and wake up right on the mark.
fn get_sleep_chunk(remaining: Duration) -> Duration {
    if remaining > Duration::from_secs(10) {
        Duration::from_secs(10)
    } else if remaining > Duration::from_secs(5) {
        Duration::from_secs(5)
    } else if remaining > Duration::from_secs(1) {
        Duration::from_secs(1)
    } else {
        remaining
    }
}

fn get_now_micros() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_tick_is_strictly_in_the_future_and_aligned() {
        let interval_micros = ExactTimerInterval::Every15Seconds.get_duration_micros();

        for now in [
            0u64,
            1,
            14_999_999,
            15_000_000,
            15_000_001,
            1_752_000_000_000_000,
            1_752_000_000_123_456,
        ] {
            let next = get_next_tick_micros(now, interval_micros);
            assert!(next > now, "next {} must be strictly after now {}", next, now);
            assert_eq!(next % interval_micros, 0, "next {} must be aligned", next);
            assert!(
                next - now <= interval_micros,
                "we must never wait more than one full interval"
            );
        }
    }

    #[test]
    fn on_the_mark_jumps_to_the_following_one() {
        // Finishing exactly on a mark must schedule the NEXT mark, never re-fire
        // the current one.
        let d = ExactTimerInterval::Every5Seconds.get_duration_micros();
        assert_eq!(get_next_tick_micros(0, d), 5_000_000);
        assert_eq!(get_next_tick_micros(5_000_000, d), 10_000_000);
        assert_eq!(get_next_tick_micros(10_000_000, d), 15_000_000);
    }

    #[test]
    fn marks_land_on_natural_wall_clock_seconds() {
        // For any "now", the 15s mark falls on a second divisible by 15.
        let d = ExactTimerInterval::Every15Seconds.get_duration_micros();
        for now in [1_752_003_123_456u64, 999_999_999, 61_000_000, 123_456_789] {
            let next = get_next_tick_micros(now, d);
            let second_in_minute = (next / 1_000_000) % 60;
            assert_eq!(second_in_minute % 15, 0);
        }
    }

    #[test]
    fn sleep_ladder_never_overshoots_and_steps_down() {
        assert_eq!(get_sleep_chunk(Duration::from_secs(30)), Duration::from_secs(10));
        assert_eq!(get_sleep_chunk(Duration::from_secs(10)), Duration::from_secs(5));
        assert_eq!(get_sleep_chunk(Duration::from_secs(6)), Duration::from_secs(5));
        assert_eq!(get_sleep_chunk(Duration::from_secs(5)), Duration::from_secs(1));
        assert_eq!(get_sleep_chunk(Duration::from_millis(1500)), Duration::from_secs(1));

        // Under one second -> sleep exactly what remains, in one shot.
        assert_eq!(get_sleep_chunk(Duration::from_secs(1)), Duration::from_secs(1));
        assert_eq!(get_sleep_chunk(Duration::from_millis(999)), Duration::from_millis(999));
        assert_eq!(get_sleep_chunk(Duration::from_millis(500)), Duration::from_millis(500));
        assert_eq!(get_sleep_chunk(Duration::from_millis(7)), Duration::from_millis(7));
        assert_eq!(get_sleep_chunk(Duration::from_micros(250)), Duration::from_micros(250));

        // The chunk must never be larger than what remains.
        for ms in [1u64, 9, 10, 11, 99, 100, 101, 999, 1000, 5000, 5001, 9999, 60_000] {
            let remaining = Duration::from_millis(ms);
            assert!(get_sleep_chunk(remaining) <= remaining);
        }
    }

    #[test]
    fn steady_state_keeps_the_target_fixed() {
        let d = ExactTimerInterval::Every30Minutes.get_duration_micros();
        let target = get_next_tick_micros(1_000_000_000, d); // some future mark

        // As `now` advances toward the mark, the target must not move.
        for now in [1_000_000_000u64, 1_000_500_000, target - 1] {
            assert_eq!(realign_target_on_clock_jump(target, now, d), target);
        }
        // Exactly one interval away is still legitimate (not a jump).
        let now = target - d;
        assert_eq!(realign_target_on_clock_jump(target, now, d), target);
    }

    #[test]
    fn backward_clock_jump_realigns_to_a_near_mark() {
        let d = ExactTimerInterval::Every30Minutes.get_duration_micros();
        let now_before = 12 * 3_600_000_000u64; // 12:00:00 since epoch-ish origin
        let target = get_next_tick_micros(now_before, d); // 12:30:00 mark

        // Clock steps back 2 hours -> target is now > one interval away.
        let now_after = now_before - 2 * 3_600_000_000u64; // 10:00:00
        let realigned = realign_target_on_clock_jump(target, now_after, d);

        assert!(realigned < target, "must re-align to a nearer mark");
        assert!(
            realigned - now_after <= d,
            "wait must be bounded to one interval after a jump"
        );
        assert_eq!(realigned % d, 0, "re-aligned target stays on a mark");
        assert_eq!(realigned, get_next_tick_micros(now_after, d));
    }
}
