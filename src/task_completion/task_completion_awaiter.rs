use tokio::sync::oneshot::Receiver;

#[derive(Clone, Copy, Debug)]
pub enum CompletionEvent<OkResult, ErrorResult> {
    Ok(OkResult),
    Error(ErrorResult),
}

pub enum TaskCompletionAwaiter<OkResult, ErrorResult> {
    Awaiting(Receiver<CompletionEvent<OkResult, ErrorResult>>),
    Completed(Result<OkResult, ErrorResult>),
}

impl<OkResult, ErrorResult> TaskCompletionAwaiter<OkResult, ErrorResult> {
    pub fn new(receiver: Receiver<CompletionEvent<OkResult, ErrorResult>>) -> Self {
        Self::Awaiting(receiver)
    }

    pub fn create_completed(result: Result<OkResult, ErrorResult>) -> Self {
        Self::Completed(result)
    }

    pub async fn get_result(self) -> Result<OkResult, ErrorResult> {
        match self {
            TaskCompletionAwaiter::Awaiting(receiver) => {
                let result = receiver.await;

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
            TaskCompletionAwaiter::Completed(completed) => completed,
        }
    }
}
