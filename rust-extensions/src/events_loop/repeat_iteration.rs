/// What the events loop reader must do once `tick()` returned.
///
/// The model is given to the tick **by ownership** - there is no clone and no
/// `&TModel` borrow to keep alive. So an iteration which is not done yet has to
/// hand the model back: that is what `Yes` carries.
pub enum RepeatIteration<TModel> {
    /// The event is **not** served yet - here is the very same model back.
    /// The loop starts one more iteration with it, with a fresh
    /// `iteration_timeout` window.
    Yes(TModel),
    /// The iteration is complete - the model is consumed and the loop goes for
    /// the next event.
    No,
}

impl<TModel> RepeatIteration<TModel> {
    pub fn is_yes(&self) -> bool {
        match self {
            RepeatIteration::Yes(_) => true,
            RepeatIteration::No => false,
        }
    }

    /// The model which was given back, if the tick asked for another iteration.
    pub fn into_model(self) -> Option<TModel> {
        match self {
            RepeatIteration::Yes(model) => Some(model),
            RepeatIteration::No => None,
        }
    }
}
