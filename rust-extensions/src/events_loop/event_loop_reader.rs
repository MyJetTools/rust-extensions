use std::{panic::AssertUnwindSafe, sync::Arc, time::Duration};

use crate::{ApplicationStates, Logger};

use super::{events_loop::EventsLoopInner, RepeatIteration};

use futures::FutureExt;

pub async fn events_loop_reader<TModel: Send + Sync + 'static>(
    name: Arc<String>,
    inner: EventsLoopInner<TModel>,
    app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
    logger: Arc<dyn Logger + Send + Sync + 'static>,
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
        let Some(message) = receiver.recv().await else {
            // The publisher is gone - there is nobody left to send an event.
            break;
        };

        let message = match message {
            super::EventsLoopMessage::NewMessage(message) => message,
            super::EventsLoopMessage::Shutdown => {
                break;
            }
        };

        // The event is owned here and is only lent to the tick - so it outlives
        // a panicked or a timed out iteration and can be served again.
        loop {
            let timer_tick_future = AssertUnwindSafe(event_loop_tick.tick(&message)).catch_unwind();

            match tokio::time::timeout(iteration_timeout, timer_tick_future).await {
                Ok(Ok(RepeatIteration::No)) => {
                    // The event is served - go for the next one.
                    break;
                }
                Ok(Ok(RepeatIteration::Yes)) => {
                    // The tick left the iteration on purpose and asked for another
                    // one - the event is not served yet, so it is not dropped.
                }
                Ok(Err(_panic)) => {
                    logger.write_error(
                        format!("EventLoop {} iteration", name.as_str()),
                        format!("Iteration is panicked. Event is going to be executed again"),
                        None.into(),
                    );
                }
                Err(_elapsed) => {
                    logger.write_error(
                        format!("EventLoop {} iteration", name.as_str()),
                        format!("Iteration is time outed. Event is going to be executed again"),
                        None.into(),
                    );
                }
            }

            // The only way out of an event which keeps panicking (or keeps asking
            // for one more iteration) - otherwise the loop is stopped by the tick.
            if app_states.is_shutting_down() {
                break;
            }
        }
    }

    let _ = AssertUnwindSafe(event_loop_tick.finished())
        .catch_unwind()
        .await;
}
