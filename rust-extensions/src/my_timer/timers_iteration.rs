use std::{panic::AssertUnwindSafe, sync::Arc, time::Duration};

use futures::FutureExt;

use crate::Logger;

use super::{MyTimerTick, RepeatTimerIteration};

pub type RegisteredTimer = (String, Arc<dyn MyTimerTick + Send + Sync + 'static>);

/// The names of the registered ticks, comma separated. A timer has no name of
/// its own - this is what tells one of them from another in a panic message.
pub fn get_timer_names(timers: &[RegisteredTimer]) -> String {
    let names: Vec<&str> = timers.iter().map(|(name, _)| name.as_str()).collect();
    names.join(", ")
}

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
///
/// A tick which overruns `iteration_timeout` is cancelled - left running, it
/// would overlap with its own next execution.
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

        // On a timeout the future is dropped right here, which cancels the tick.
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
            Err(_) => report_timeout(timer_id, iteration_timeout, logger),
        }

        return repeat_immediately;
    }

    // One deadline for the whole pass: the ticks run in parallel, so each of
    // them gets `iteration_timeout` from the start, however long the others take.
    let deadline = tokio::time::Instant::now() + iteration_timeout;

    let mut timer_handles = Vec::with_capacity(timers.len());
    for timer in timers {
        let handle = tokio::spawn(execute_timer(timer.1.clone()));
        timer_handles.push((*timer, handle));
    }

    for (timer, mut timer_handler) in timer_handles {
        let timer_id = &timer.0;

        match tokio::time::timeout_at(deadline, &mut timer_handler).await {
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
            Err(_) => {
                // Dropping a JoinHandle does not stop the task - it has to be aborted.
                timer_handler.abort();
                report_timeout(timer_id, iteration_timeout, logger);
            }
        }
    }

    repeat_immediately
}

fn report_timeout(
    timer_id: &str,
    iteration_timeout: Duration,
    logger: &Arc<dyn Logger + Send + Sync + 'static>,
) {
    let message = format!(
        "Timer {} is cancelled: it did not finish within {:?}",
        timer_id, iteration_timeout
    );
    println!("{}", message);
    logger.write_error(timer_id.to_string(), message, None);
}

pub async fn execute_timer(
    timer: Arc<dyn MyTimerTick + Send + Sync + 'static>,
) -> RepeatTimerIteration {
    timer.tick().await
}
