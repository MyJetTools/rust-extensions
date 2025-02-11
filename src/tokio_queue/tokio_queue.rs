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

        // Check if there's data available
        if queue_access.len() == 0 {
            // No data, register waker and return Pending

            self.inner.waker.register(cx.waker());

            println!("Return pending");

            return Poll::Pending;
        }

        // Read available data into the buffer

        println!("Remaining: {}", buf.remaining());

        if buf.remaining() >= queue_access.len() {
            buf.put_slice(queue_access.drain(..).as_slice());
        } else {
            buf.put_slice(queue_access.drain(..buf.remaining()).as_slice());
        }

        Poll::Ready(Ok(()))
    }
}
