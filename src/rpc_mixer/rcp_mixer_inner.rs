use crate::TaskCompletion;
use std::sync::Arc;
pub struct Request<
    TItem: Send + Sync + 'static,
    TResult: Send + Sync + 'static,
    TError: Send + Sync + 'static,
> {
    pub item: TItem,
    pub completion: TaskCompletion<TResult, Arc<TError>>,
}

pub struct RpcMixerInner<
    TItem: Send + Sync + 'static,
    TResult: Send + Sync + 'static,
    TError: Send + Sync + 'static,
> {
    pub receiver: Option<tokio::sync::mpsc::UnboundedReceiver<()>>,
    pub queue: Vec<Request<TItem, TResult, TError>>,
}

impl<
        TItem: Send + Sync + 'static,
        TResult: Send + Sync + 'static,
        TError: Send + Sync + 'static,
    > RpcMixerInner<TItem, TResult, TError>
{
    pub fn new(receiver: tokio::sync::mpsc::UnboundedReceiver<()>) -> Self {
        Self {
            receiver: Some(receiver),
            queue: Vec::new(),
        }
    }
}
