use std::{panic::AssertUnwindSafe, sync::Arc, time::Duration};

use futures::FutureExt;

use crate::Logger;

use super::{MyTimerTick, RepeatTimerIteration};

pub type RegisteredTimer = (String, Arc<dyn MyTimerTick + Send + Sync + 'static>);

/// Runs a single pass over `timers` and returns the ones which asked to be
/// repeated immediately.
///
/// Shared by [`MyTimer`](crate::MyTimer) and
/// [`MyExactTimer`](crate::MyExactTimer) - they differ in how they wait for the
/// next tick, not in how they execute one.
///
/// Every pass gets its own `iteration_timeout` window, so a tick which leaves
/// its iteration on purpose (`RepeatTimerIteration::Immediately`) starts the
/// next one with the timeout budget reset.
///
/// Only the ticks which asked for it are repeated: a tick that answered
/// `WithInterval` keeps its schedule and is not dragged into a neighbour's extra
/// pass. A tick which panicked or timed out answered nothing and is not
/// repeated either.
pub async fn execute_timers_iteration<'s>(
    timers: &[&'s RegisteredTimer],
    logger: &Arc<dyn Logger + Send + Sync + 'static>,
    iteration_timeout: Duration,
) -> Vec<&'s RegisteredTimer> {
    let mut repeat_immediately = Vec::new();

    if timers.len() == 1 {
        let timer = timers[0];
        let (timer_id, timer_tick) = timer;
        let tick_future = AssertUnwindSafe(execute_timer(timer_tick.clone())).catch_unwind();

        match tokio::time::timeout(iteration_timeout, tick_future).await {
            Ok(Ok(repeat)) => {
                if repeat.is_immediately() {
                    repeat_immediately.push(timer);
                }
            }
            Ok(Err(_panic)) => {
                let message = format!("Timer {} is panicked", timer_id);
                println!("{}", message);
                logger.write_error(timer_id.to_string().into(), message.into(), None.into());
            }
            Err(err) => {
                println!("Timer {} is time outed with err: {:?}", timer_id, err);
            }
        }

        return repeat_immediately;
    }

    let mut timer_handles = Vec::with_capacity(timers.len());
    for timer in timers {
        let handle = tokio::spawn(execute_timer(timer.1.clone()));
        timer_handles.push((*timer, handle));
    }

    for (timer, timer_handler) in timer_handles {
        let timer_id = &timer.0;

        match tokio::time::timeout(iteration_timeout, timer_handler).await {
            Ok(Ok(repeat)) => {
                if repeat.is_immediately() {
                    repeat_immediately.push(timer);
                }
            }
            Ok(Err(err)) => {
                let message = format!("Timer {} is panicked. Err: {:?}", timer_id, err);
                let timer_id = timer_id.to_string();
                let logger = logger.clone();

                tokio::spawn(async move {
                    println!("{}", message);
                    logger.write_error(timer_id.into(), message.into(), None.into());
                });
            }
            Err(err) => {
                println!("Timer {} is time outed with err: {:?}", timer_id, err);
            }
        }
    }

    repeat_immediately
}

pub async fn execute_timer(
    timer: Arc<dyn MyTimerTick + Send + Sync + 'static>,
) -> RepeatTimerIteration {
    timer.tick().await
}
