#[derive(Default)]
pub struct AsyncWaker {
    sender: Option<tokio::sync::oneshot::Sender<()>>,
}

impl AsyncWaker {
    pub fn wake(&mut self) {
        if let Some(value) = self.sender.take() {
            let _ = value.send(());
        }
    }

    pub fn get_awaiter(&mut self) -> AsyncWakerAwaiter {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.sender = Some(sender);
        AsyncWakerAwaiter { receiver }
    }
}

pub struct AsyncWakerAwaiter {
    receiver: tokio::sync::oneshot::Receiver<()>,
}

impl AsyncWakerAwaiter {
    pub async fn await_me(self) {
        let _ = self.receiver.await;
    }
}
