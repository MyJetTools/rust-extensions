use std::sync::Mutex;

use futures::task::AtomicWaker;

pub struct TokioQueuePublish {
    pub(crate) queue: Mutex<Vec<u8>>,
    pub(crate) waker: AtomicWaker,
}

impl TokioQueuePublish {
    pub fn enqueue(&self, payload: &[u8]) {
        let mut write_access = self.queue.lock().unwrap();
        write_access.extend_from_slice(payload);
        drop(write_access);
        self.waker.wake();
    }
}

impl Default for TokioQueuePublish {
    fn default() -> Self {
        Self {
            queue: Mutex::default(),
            waker: AtomicWaker::new(),
        }
    }
}
