/// What the reader must do once `execute()` returned.
///
/// A job which understood in the middle of its work that there is more to do -
/// but does not want to keep the current iteration running (an external timeout,
/// a batch limit, a fairness cap) - returns `Yes`: it leaves the iteration and
/// asks to be started again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatIteration {
    /// Run `execute()` again straight away. The trigger which is being served
    /// right now is **not** consumed - the counter stays as it is.
    Yes,
    /// The iteration is complete - consume the trigger and stop when the
    /// counter is drained.
    No,
}

impl RepeatIteration {
    pub fn is_yes(&self) -> bool {
        match self {
            RepeatIteration::Yes => true,
            RepeatIteration::No => false,
        }
    }
}

#[async_trait::async_trait]
pub trait BackgroundJob: Send + Sync + 'static {
    async fn execute(&self) -> RepeatIteration;
}
