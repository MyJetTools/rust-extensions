pub struct RoundTripPusherInner<TItem: Send + Sync + 'static> {
    pub receiver: Option<tokio::sync::mpsc::UnboundedReceiver<()>>,
    pub queue: Vec<TItem>,
}

impl<TItem: Send + Sync + 'static> RoundTripPusherInner<TItem> {
    pub fn new(receiver: tokio::sync::mpsc::UnboundedReceiver<()>) -> Self {
        Self {
            receiver: Some(receiver),
            queue: Vec::new(),
        }
    }
}
