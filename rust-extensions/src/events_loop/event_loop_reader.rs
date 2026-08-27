use std::{panic::AssertUnwindSafe, sync::Arc, time::Duration};

use crate::{ApplicationStates, Logger};

use super::{events_loop::EventsLoopInner, RepeatIteration};

use futures::FutureExt;

pub async fn events_loop_reader<TModel: Send + 'static>(
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

        // The model is moved into the tick - not cloned, not borrowed. It comes
        // back inside RepeatIteration::Yes when the tick wants one more go, and
        // dies with the future when the iteration panicked or timed out.
        let mut model = message;

        loop {
            let timer_tick_future = AssertUnwindSafe(event_loop_tick.tick(model)).catch_unwind();

            match tokio::time::timeout(iteration_timeout, timer_tick_future).await {
                Ok(Ok(RepeatIteration::No)) => {
                    // The event is served - go for the next one.
                    break;
                }
                Ok(Ok(RepeatIteration::Yes(the_same_model))) => {
                    // The tick left the iteration on purpose and gave the event
                    // back - the very same model goes into the next iteration.
                    model = the_same_model;
                }
                Ok(Err(_panic)) => {
                    logger.write_error(
                        format!("EventLoop {} iteration", name.as_str()),
                        format!("Iteration is panicked. Event is lost"),
                        None.into(),
                    );
                    break;
                }
                Err(_elapsed) => {
                    logger.write_error(
                        format!("EventLoop {} iteration", name.as_str()),
                        format!("Iteration is time outed. Event is lost"),
                        None.into(),
                    );
                    break;
                }
            }

            // The only way out of an event whose tick keeps asking for one more
            // iteration - otherwise the loop is stopped by the tick itself.
            if app_states.is_shutting_down() {
                break;
            }
        }
    }

    let _ = AssertUnwindSafe(event_loop_tick.finished())
        .catch_unwind()
        .await;
}
