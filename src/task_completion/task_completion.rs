use tokio::sync::oneshot::{Receiver, Sender};

use super::{CompletionEvent, TaskCompletionAwaiter};

#[derive(Debug)]
pub enum TaskCompletionError {
    CanNotSetOkResult(String),
    CanNotSetErrorResult(String),
}

pub struct TaskCompletion<OkResult, ErrorResult> {
    pub receiver: Option<Receiver<CompletionEvent<OkResult, ErrorResult>>>,
    pub sender: Option<Sender<CompletionEvent<OkResult, ErrorResult>>>,
}

impl<OkResult, ErrorResult> TaskCompletion<OkResult, ErrorResult> {
    pub fn new() -> Self {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        Self {
            receiver: Some(receiver),
            sender: Some(sender),
        }
    }

    fn get_sender(&mut self) -> Option<Sender<CompletionEvent<OkResult, ErrorResult>>> {
        let mut new_result = None;
        std::mem::swap(&mut new_result, &mut self.sender);
        new_result
    }

    fn get_receiver(&mut self) -> Option<Receiver<CompletionEvent<OkResult, ErrorResult>>> {
        let mut new_result = None;
        std::mem::swap(&mut new_result, &mut self.receiver);
        new_result
    }

    pub fn set_ok(&mut self, result: OkResult) {
        let sender = self.get_sender();

        match sender {
            Some(sender) => match sender.send(CompletionEvent::Ok(result)) {
                Ok(_) => {
                    return;
                }
                Err(_) => {
                    panic!("Can not set Ok result to the task completion.");
                }
            },
            None => {
                panic!("You are trying to set Ok as a result for a second time");
            }
        }
    }

    pub fn try_set_ok(&mut self, result: OkResult) -> Result<(), TaskCompletionError> {
        let sender = self.get_sender();

        match sender {
            Some(sender) => match sender.send(CompletionEvent::Ok(result)) {
                Ok(_) => {
                    return Ok(());
                }
                Err(_) => {
                    return Err(TaskCompletionError::CanNotSetOkResult(
                        "Can not set Ok result to the task completion.".to_string(),
                    ));
                }
            },
            None => {
                return Err(TaskCompletionError::CanNotSetOkResult(
                    "You are trying to set Ok as a result for a second time.".to_string(),
                ));
            }
        }
    }

    pub fn set_error(&mut self, result: ErrorResult) {
        let sender = self.get_sender();

        match sender {
            Some(sender) => {
                let result = sender.send(CompletionEvent::Error(result));
                if let Err(_) = result {
                    panic!("Can not set Error result to the task completion. ");
                }
            }
            None => {
                panic!("You are trying to set error as a result for a second time");
            }
        }
    }

    pub fn set_panic(&mut self, message: String) {
        let sender = self.get_sender();

        match sender {
            Some(sender) => {
                let result = sender.send(CompletionEvent::Panic(message));
                if let Err(_) = result {
                    panic!("Can not set Error result to the task completion. ");
                }
            }
            None => {
                panic!("You are trying to set error as a result for a second time");
            }
        }
    }

    pub fn try_set_error(&mut self, result: ErrorResult) -> Result<(), TaskCompletionError> {
        let sender = self.get_sender();

        match sender {
            Some(sender) => {
                let result = sender.send(CompletionEvent::Error(result));
                if let Err(_) = result {
                    return Err(TaskCompletionError::CanNotSetErrorResult(
                        "Can not set Error result to the task completion.".to_string(),
                    ));
                } else {
                    return Ok(());
                }
            }
            None => {
                return Err(TaskCompletionError::CanNotSetErrorResult(
                    "You are trying to set error as a result for a second time.".to_string(),
                ));
            }
        }
    }

    pub fn get_awaiter(&mut self) -> TaskCompletionAwaiter<OkResult, ErrorResult> {
        let receiver = self.get_receiver();

        match receiver {
            Some(receiver) => {
                return TaskCompletionAwaiter::new(receiver);
            }
            None => {
                panic!("You are trying to get awaiter for the second time");
            }
        }
    }
}
