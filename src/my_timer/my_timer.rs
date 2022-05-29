use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::ApplicationStates;

use super::{MyTimerLogger, MyTimerTick};

pub struct MyTimer {
    interval: Duration,
    timers: HashMap<String, Arc<dyn MyTimerTick + Send + Sync + 'static>>,
    iteration_timeout: Duration,
}

impl MyTimer {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            timers: HashMap::new(),
            iteration_timeout: Duration::from_secs(60),
        }
    }

    pub fn new_with_execute_timeout(interval: Duration, iteration_timeout: Duration) -> Self {
        Self {
            interval,
            timers: HashMap::new(),
            iteration_timeout,
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

    pub fn start<TLogger: MyTimerLogger + Send + Sync + 'static>(
        &self,
        app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
        logger: Arc<TLogger>,
    ) {
        let timers = self.timers.clone();
        tokio::spawn(timer_loop(
            timers,
            self.interval,
            app_states,
            logger,
            self.iteration_timeout,
        ));
    }
}

async fn timer_loop<TLogger: MyTimerLogger + Send + Sync + 'static>(
    timers: HashMap<String, Arc<dyn MyTimerTick + Send + Sync + 'static>>,
    interval: Duration,
    app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
    logger: Arc<TLogger>,
    iteration_timeout: Duration,
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

        logger.write_info(timer_id.to_string(), message);
    }

    while !app_states.is_shutting_down() {
        tokio::time::sleep(interval).await;

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
                            logger.write_error(timer_id.to_string(), message);
                        });
                    }
                }
                Err(err) => {
                    println!("Timer {} is timeouted with err: {:?}", timer_id, err);
                }
            }
        }
    }
}

async fn execute_timer(timer: Arc<dyn MyTimerTick + Send + Sync + 'static>) {
    timer.tick().await;
}
