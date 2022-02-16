use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::ApplicationStates;

use super::MyTimerTick;

pub struct MyTimerLogEvent {
    pub timer_id: String,
    pub message: String,
}

pub struct MyTimer {
    interval: Duration,
    timers: Vec<Arc<dyn MyTimerTick + Send + Sync + 'static>>,
}

impl MyTimer {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            timers: Vec::new(),
        }
    }

    pub fn register_timer(&mut self, my_timer_tick: Arc<dyn MyTimerTick + Send + Sync + 'static>) {
        self.timers.push(my_timer_tick);
    }

    pub fn start<TLogger: Send + Sync + 'static + Fn(MyTimerLogEvent)>(
        &self,
        app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
        logger: Arc<TLogger>,
    ) {
        let timers = self.timers.clone();
        tokio::spawn(timer_loop(timers, self.interval, app_states, logger));
    }
}

async fn timer_loop<TLogger: Send + Sync + 'static + Fn(MyTimerLogEvent)>(
    timers: Vec<Arc<dyn MyTimerTick + Send + Sync + 'static>>,
    interval: Duration,
    app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
    logger: Arc<TLogger>,
) {
    while !app_states.is_initialized() {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    for timer in &timers {
        let message = format!(
            "Timer {} is started with delay {} sec",
            timer.get_name(),
            interval.as_secs()
        );
        logger(MyTimerLogEvent {
            timer_id: timer.get_name().to_string(),
            message,
        });
    }

    while !app_states.is_shutting_down() {
        tokio::time::sleep(interval).await;

        let mut timer_handles = HashMap::new();
        for timer in &timers {
            let handle = tokio::spawn(execute_timer(timer.clone()));
            timer_handles.insert(timer.get_name(), handle);
        }

        for (id, timer_handler) in timer_handles {
            let result = timer_handler.await;

            if let Err(err) = result {
                let message = format!("Timer {} is paniced {:?}", id, err);
                logger(MyTimerLogEvent {
                    timer_id: id.to_string(),
                    message,
                });
            }
        }
    }
}

async fn execute_timer(timer: Arc<dyn MyTimerTick + Send + Sync + 'static>) {
    timer.tick().await;
}
