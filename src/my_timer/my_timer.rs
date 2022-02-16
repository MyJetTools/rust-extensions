use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::ApplicationStates;

use super::MyTimerTick;

pub enum MyTimerLogEventLevel {
    Info,
    FatalError,
}

pub struct MyTimerLogEvent {
    pub level: MyTimerLogEventLevel,
    pub timer_id: String,
    pub message: String,
}

pub struct MyTimer {
    interval: Duration,
    timers: HashMap<String, Arc<dyn MyTimerTick + Send + Sync + 'static>>,
}

impl MyTimer {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            timers: HashMap::new(),
        }
    }

    pub fn register_timer(
        &mut self,
        name: &str,
        my_timer_tick: Arc<dyn MyTimerTick + Send + Sync + 'static>,
    ) {
        if self.timers.contains_key(name) {
            panic!("Timer with the name [{}] is already registered", name);
        }
        self.timers.insert(name.to_string(), my_timer_tick);
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
    timers: HashMap<String, Arc<dyn MyTimerTick + Send + Sync + 'static>>,
    interval: Duration,
    app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
    logger: Arc<TLogger>,
) {
    while !app_states.is_initialized() {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    for timer_id in timers.keys() {
        let message = format!(
            "Timer {} is started with delay {} sec",
            timer_id,
            interval.as_secs()
        );
        logger(MyTimerLogEvent {
            level: MyTimerLogEventLevel::Info,
            timer_id: timer_id.to_string(),
            message,
        });
    }

    while !app_states.is_shutting_down() {
        tokio::time::sleep(interval).await;

        let mut timer_handles = HashMap::new();
        for (timer_id, timer) in &timers {
            let handle = tokio::spawn(execute_timer(timer.clone()));
            timer_handles.insert(timer_id, handle);
        }

        for (timer_id, timer_handler) in timer_handles {
            let result = timer_handler.await;

            if let Err(err) = result {
                let message = format!("Timer {} is panicked. Err: {:?}", timer_id, err);
                logger(MyTimerLogEvent {
                    level: MyTimerLogEventLevel::FatalError,
                    timer_id: timer_id.to_string(),
                    message,
                });
            }
        }
    }
}

async fn execute_timer(timer: Arc<dyn MyTimerTick + Send + Sync + 'static>) {
    timer.tick().await;
}
