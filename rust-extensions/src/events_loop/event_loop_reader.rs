use std::{ panic::AssertUnwindSafe, sync::Arc, time::Duration};

use crate::{ApplicationStates, Logger};

use super::{events_loop::EventsLoopInner};

use futures::FutureExt;

pub async fn events_loop_reader<TModel : Send + 'static>(
    name: Arc<String>,
    inner: EventsLoopInner<TModel>,
    app_states: Arc<dyn ApplicationStates + Send +  Sync+ 'static>,
    logger: Arc<dyn Logger + Send + Sync+ 'static>,
    iteration_timeout: Duration,

) {
    let EventsLoopInner {
        event_loop_tick,
        mut receiver,
    } = inner;

    while !app_states.is_initialized() {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    let _ = AssertUnwindSafe(event_loop_tick.started())
    .catch_unwind()
    .await;

    while !app_states.is_shutting_down() {
        if let Some(message) = tokio::sync::mpsc::UnboundedReceiver::recv(&mut receiver).await {

            let message = match message{
                super::EventsLoopMessage::NewMessage(message) => message,
                super::EventsLoopMessage::Shutdown => {
                    break;
                },
            };

            let timeout_tick = event_loop_tick.tick(message);

            let timer_tick_future = AssertUnwindSafe(timeout_tick)
                    .catch_unwind();

                match tokio::time::timeout(iteration_timeout, timer_tick_future).await {
                Ok(Ok(_)) => {
              
                }
                Ok(Err(_panic)) => {
                      logger.write_error(
                            format!("EventLoop {} iteration", name.as_str()),
                            format!("Iteration is panicked"),
                            None.into(),
                        );
                }
                Err(_elapsed) => {
                    logger.write_error(
                        format!("EventLoop {} iteration", name.as_str()),
                        format!("Iteration is time outed"),
                        None.into(),
                    );
                }
            }

        }
    }

  
    let _ = AssertUnwindSafe(event_loop_tick.finished())
    .catch_unwind()
    .await;
}
