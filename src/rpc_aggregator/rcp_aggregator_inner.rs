use crate::TaskCompletion;
use std::sync::Arc;

pub struct Request<TItem: Send + Sync + 'static, TError: Send + Sync + 'static> {
    pub request_data: TItem,
    pub completion: TaskCompletion<(), Arc<TError>>,
}

pub struct RpcAggregatorInner<TItem: Send + Sync + 'static, TError: Send + Sync + 'static> {
    pub receiver: Option<tokio::sync::mpsc::UnboundedReceiver<()>>,
    pub queue: Vec<Request<TItem, TError>>,
}

impl<TItem: Send + Sync + 'static, TError: Send + Sync + 'static>
    RpcAggregatorInner<TItem, TError>
{
    pub fn new(receiver: tokio::sync::mpsc::UnboundedReceiver<()>) -> Self {
        Self {
            receiver: Some(receiver),
            queue: Vec::new(),
        }
    }
}
