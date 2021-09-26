use tokio::sync::oneshot::Receiver;

#[derive(Clone, Copy, Debug)]
pub enum CompletionEvent<OkResult, ErrorResult> {
    Ok(OkResult),
    Error(ErrorResult),
}

pub struct TaskCompletionAwaiter<OkResult, ErrorResult> {
    pub receiver: Receiver<CompletionEvent<OkResult, ErrorResult>>,
}

impl<OkResult, ErrorResult> TaskCompletionAwaiter<OkResult, ErrorResult> {
    pub fn new(receiver: Receiver<CompletionEvent<OkResult, ErrorResult>>) -> Self {
        Self { receiver }
    }

    pub async fn get_result(self) -> Result<OkResult, ErrorResult> {
        let result = self.receiver.await;

        match result {
            Ok(result) => match result {
                CompletionEvent::Ok(ok) => return Ok(ok),
                CompletionEvent::Error(err) => return Err(err),
            },
            Err(error) => panic!(
                "Can not recivev result for a task completion. Err: {:?}",
                error
            ),
        }
    }
}
