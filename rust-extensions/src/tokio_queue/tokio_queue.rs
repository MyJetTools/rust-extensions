use std::{sync::Arc, task::Poll};

use super::*;

pub struct TokioQueue {
    inner: Arc<TokioQueuePublish>,
}

impl TokioQueue {
    pub fn new() -> Self {
        Self {
            inner: Arc::default(),
        }
    }

    pub fn get_publisher(&self) -> Arc<TokioQueuePublish> {
        self.inner.clone()
    }
}

impl tokio::io::AsyncRead for TokioQueue {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let mut queue_access = self.inner.queue.lock().unwrap();

        if queue_access.is_empty() {
            // Register the waker while holding the lock to avoid races,
            // then re-check and drop the lock before returning Pending.
            self.inner.waker.register(cx.waker());

            if queue_access.is_empty() {
                drop(queue_access);
                return Poll::Pending;
            }
        }

        let available = queue_access.len();
        let to_copy = available.min(buf.remaining());

        buf.put_slice(&queue_access[..to_copy]);
        queue_access.drain(..to_copy);

        Poll::Ready(Ok(()))
    }
}
