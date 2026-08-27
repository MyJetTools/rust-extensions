use std::{sync::Arc, time::Duration};

use parking_lot::Mutex;

use crate::{ApplicationStates, Logger, StrOrString};

use super::{EventsLoopPublisher, EventsLoopTick};

pub enum EventsLoopMessage<TModel> {
    NewMessage(TModel),
    Shutdown,
}

impl<TModel: 'static> EventsLoopMessage<TModel> {
    pub fn is_shutdown(&self) -> bool {
        match self {
            EventsLoopMessage::Shutdown => true,
            _ => false,
        }
    }

    pub fn unwrap_message(self) -> TModel {
        match self {
            EventsLoopMessage::NewMessage(message) => message,
            _ => panic!("EventsLoopMessage::unwrap_message() called on a non-NewMessage message"),
        }
    }
}

pub(super) struct EventsLoopInner<TModel: Send + 'static> {
    pub event_loop_tick: Arc<dyn EventsLoopTick<TModel> + Send + Sync + 'static>,
    pub receiver: tokio::sync::mpsc::UnboundedReceiver<EventsLoopMessage<TModel>>,
}

pub struct EventsLoop<TModel: Send + 'static> {
    pending_receiver:
        Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<EventsLoopMessage<TModel>>>>,
    inner: Mutex<Option<EventsLoopInner<TModel>>>,
    publisher: EventsLoopPublisher<TModel>,
    name: Arc<String>,
    iteration_timeout: Duration,
}

impl<TModel: Send + 'static> EventsLoop<TModel> {
    pub fn new(name: impl Into<StrOrString<'static>>) -> Self {
        let name: Arc<String> = Arc::new(name.into().to_string());

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        Self {
            publisher: EventsLoopPublisher::new(name.clone(), sender),
            name,
            iteration_timeout: Duration::from_secs(30),
            pending_receiver: Mutex::new(Some(receiver)),
            inner: Mutex::new(None),
        }
    }

    pub fn set_iteration_timeout(mut self, timeout: Duration) -> Self {
        self.iteration_timeout = timeout;
        self
    }

    pub fn register_event_loop(
        &self,
        event_loop: Arc<dyn EventsLoopTick<TModel> + Send + Sync+  'static>,
    ) {
        let receiver = self.pending_receiver.lock().take();

        if receiver.is_none() {
            panic!(
                "Event loop tick is already registered for this event loop {}",
                self.name
            );
        }

        let mut inner_lock = self.inner.lock();
        *inner_lock = Some(EventsLoopInner {
            event_loop_tick: event_loop,
            receiver: receiver.unwrap(),
        });
    }

    pub fn start(
        &self,
        app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
        logger: Arc<dyn Logger + Send + Sync + 'static>,
    ) {
        let inner = self.inner.lock().take();

        let Some(inner) = inner else{
             panic!(
                "Event Loop {} is not registered or already started.",
                self.name
            );
        };


        tokio::spawn(super::event_loop_reader::events_loop_reader(
            self.name.clone(),
            inner,
            app_states,
            logger,
            self.iteration_timeout,
        ));
    }

    pub fn get_publisher(&self) -> EventsLoopPublisher<TModel> {
        self.publisher.clone()
    }

    pub fn send(&self, model: TModel) {
        self.publisher.send(model);
    }

    pub fn stop(&self) {
        self.publisher.stop();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    use parking_lot::Mutex;

    use crate::events_loop::{EventsLoopTick, RepeatIteration};
    use crate::{AppStates, Logger};

    use super::EventsLoop;

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
    }

    struct TestLogger;

    impl Logger for TestLogger {
        fn write_info(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
        fn write_warning(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
        fn write_error(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
        fn write_fatal_error(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
        fn write_debug_info(&self, _: String, _: String, _: Option<HashMap<String, String>>) {}
    }

    /// Writes down every model it was given - so both the amount of the
    /// iterations and the model of each of them can be checked.
    struct RecordingTick {
        seen: Arc<Mutex<Vec<String>>>,
        /// How the tick answers on the n-th iteration. The last answer is reused
        /// once the script is over.
        script: Vec<Answer>,
    }

    #[derive(Clone, Copy)]
    enum Answer {
        Served,
        RepeatIt,
        Panic,
        TimeOut,
    }

    #[async_trait::async_trait]
    impl EventsLoopTick<String> for RecordingTick {
        async fn started(&self) {}

        async fn tick(&self, model: String) -> RepeatIteration<String> {
            let answer = {
                let mut seen = self.seen.lock();
                seen.push(model.clone());
                let no = seen.len() - 1;
                self.script[no.min(self.script.len() - 1)]
            };

            match answer {
                Answer::Served => RepeatIteration::No,
                Answer::RepeatIt => RepeatIteration::Yes(model),
                Answer::Panic => panic!("Iteration is panicked on purpose"),
                Answer::TimeOut => {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    RepeatIteration::No
                }
            }
        }

        async fn finished(&self) {}
    }

    fn start_events_loop(
        name: &'static str,
        seen: &Arc<Mutex<Vec<String>>>,
        script: Vec<Answer>,
    ) -> EventsLoop<String> {
        let events_loop =
            EventsLoop::new(name).set_iteration_timeout(Duration::from_millis(100));

        events_loop.register_event_loop(Arc::new(RecordingTick {
            seen: seen.clone(),
            script,
        }));

        events_loop.start(Arc::new(AppStates::create_initialized()), Arc::new(TestLogger));

        events_loop
    }

    async fn wait_for(seen: &Arc<Mutex<Vec<String>>>, expected: usize) {
        for _ in 0..2000 {
            if seen.lock().len() >= expected {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        panic!(
            "Expected {} iterations, got {:?}",
            expected,
            seen.lock().as_slice()
        );
    }

    #[test]
    fn each_event_is_served_once() {
        rt().block_on(async {
            let seen = Arc::new(Mutex::new(Vec::new()));
            let events_loop = start_events_loop("test-served", &seen, vec![Answer::Served]);

            events_loop.send("first".to_string());
            events_loop.send("second".to_string());
            events_loop.send("third".to_string());

            wait_for(&seen, 3).await;
            tokio::time::sleep(Duration::from_millis(50)).await;

            assert_eq!(seen.lock().as_slice(), &["first", "second", "third"]);
        });
    }

    #[test]
    fn repeat_iteration_yes_runs_the_same_event_again() {
        rt().block_on(async {
            let seen = Arc::new(Mutex::new(Vec::new()));
            let events_loop = start_events_loop(
                "test-repeat",
                &seen,
                vec![Answer::RepeatIt, Answer::RepeatIt, Answer::Served],
            );

            events_loop.send("event".to_string());
            events_loop.send("next-event".to_string());

            wait_for(&seen, 4).await;
            tokio::time::sleep(Duration::from_millis(50)).await;

            // The first event was asked to be repeated twice and was given back
            // to the tick as it is - only then the loop moved to the next one.
            assert_eq!(
                seen.lock().as_slice(),
                &["event", "event", "event", "next-event"]
            );
        });
    }

    #[test]
    fn panicked_iteration_drops_the_event() {
        rt().block_on(async {
            let seen = Arc::new(Mutex::new(Vec::new()));
            let events_loop =
                start_events_loop("test-panic", &seen, vec![Answer::Panic, Answer::Served]);

            events_loop.send("event".to_string());
            events_loop.send("next-event".to_string());

            wait_for(&seen, 2).await;
            tokio::time::sleep(Duration::from_millis(50)).await;

            // The model was moved into the tick, so the panic took it with it -
            // there is nothing left to repeat with. The event is logged and
            // dropped, and the loop moves on instead of getting stuck on it.
            assert_eq!(seen.lock().as_slice(), &["event", "next-event"]);
        });
    }

    #[test]
    fn timed_out_iteration_drops_the_event() {
        rt().block_on(async {
            let seen = Arc::new(Mutex::new(Vec::new()));
            let events_loop =
                start_events_loop("test-timeout", &seen, vec![Answer::TimeOut, Answer::Served]);

            events_loop.send("event".to_string());
            events_loop.send("next-event".to_string());

            wait_for(&seen, 2).await;
            tokio::time::sleep(Duration::from_millis(50)).await;

            // Same story as the panic: the timed out future is dropped together
            // with the model it owns.
            assert_eq!(seen.lock().as_slice(), &["event", "next-event"]);
        });
    }
}
