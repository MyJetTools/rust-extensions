use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::{ApplicationStates, Logger};

use super::MyTimerTick;

pub struct MyTimer {
    interval: Duration,
    timers: Vec<(String, Arc<dyn MyTimerTick + Send + Sync + 'static>)>,
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

async fn timer_loop(
    timers: Vec<(String, Arc<dyn MyTimerTick + Send + Sync + 'static>)>,
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

        tokio::time::sleep(interval).await;
    }
}

async fn execute_timer(timer: Arc<dyn MyTimerTick + Send + Sync + 'static>) {
    timer.tick().await;
}
