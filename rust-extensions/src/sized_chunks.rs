/// Running size of a batch being filled, which answers one question: does the
/// next item still fit, or must what is held be flushed first.
///
/// It exists because "how many items fit" and "how many bytes fit" are different
/// questions, and only the second one is ever the real limit. A gRPC message, a
/// request body, a UDP datagram, a log line, a row batch handed to a driver — each
/// is capped in BYTES, so a batch cut by item count is a guess about the average
/// item, and the guess is wrong exactly when items are unusually large, which is
/// the case nobody tests. Measure instead of estimating and there is nothing left
/// to be wrong about.
///
/// The cost function is supplied by the caller, so this stays free of any
/// serialisation dependency and both sides of a wire can use the same type:
/// `prost::Message::encoded_len`, `serde_json::to_vec(..).len()`, a `str`'s `len`,
/// or a hand-written estimate are all just `usize`.
///
/// # Empty batch always accepts
///
/// [`SizeBudget::needs_flush`] answers `false` while nothing is accumulated, so a
/// single item larger than the whole limit is admitted alone rather than being
/// refused forever. That is deliberate: an over-sized item cannot be made to fit
/// by flushing, so the alternatives are to pass it on and let the real boundary
/// reject it with its own error, or to spin. Passing it on is the honest one.
///
/// # Example
///
/// ```
/// use rust_extensions::SizeBudget;
///
/// let mut budget = SizeBudget::new(10);
/// let mut batch: Vec<&str> = Vec::new();
/// let mut sent: Vec<Vec<&str>> = Vec::new();
///
/// for word in ["four", "five5", "six666", "7seven7"] {
///     if budget.needs_flush(word.len()) {
///         sent.push(std::mem::take(&mut batch));
///         budget.reset();
///     }
///     budget.add(word.len());
///     batch.push(word);
/// }
///
/// if !batch.is_empty() {
///     sent.push(batch);
/// }
///
/// assert_eq!(vec![vec!["four", "five5"], vec!["six666"], vec!["7seven7"]], sent);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SizeBudget {
    limit: usize,
    used: usize,
}

impl SizeBudget {
    pub fn new(limit: usize) -> Self {
        Self { limit, used: 0 }
    }

    /// The ceiling this budget was created with.
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// How much has been added since the last [`SizeBudget::reset`].
    pub fn used(&self) -> usize {
        self.used
    }

    /// Nothing has been added since the last [`SizeBudget::reset`].
    pub fn is_empty(&self) -> bool {
        self.used == 0
    }

    /// Whether the batch has to be flushed before an item costing `cost` is
    /// added. Always `false` on an empty batch — see the type docs for why.
    pub fn needs_flush(&self, cost: usize) -> bool {
        !self.is_empty() && self.used.saturating_add(cost) > self.limit
    }

    /// Account for an item that is being added to the current batch.
    pub fn add(&mut self, cost: usize) {
        self.used = self.used.saturating_add(cost);
    }

    /// Start a new batch. The limit is kept.
    pub fn reset(&mut self) {
        self.used = 0;
    }
}

/// Cut `items` into batches whose measured size stays within `limit`.
///
/// The single-collection shape of [`SizeBudget`], which is what most callers
/// want. When one batch has to hold items of SEVERAL types — two `repeated`
/// fields of one protobuf message, say — drive a [`SizeBudget`] directly instead,
/// so the types share one batch rather than getting a batch each.
///
/// An item whose own size exceeds `limit` becomes a batch of one; the type docs
/// explain that choice. An empty input yields no batches at all, so a caller that
/// sends one message per batch sends nothing rather than an empty one.
///
/// # Example
///
/// ```
/// use rust_extensions::split_into_sized_chunks;
///
/// let chunks = split_into_sized_chunks(vec!["four", "five5", "six666"], 10, |it| it.len());
///
/// assert_eq!(vec![vec!["four", "five5"], vec!["six666"]], chunks);
/// ```
pub fn split_into_sized_chunks<T>(
    items: impl IntoIterator<Item = T>,
    limit: usize,
    size_of: impl Fn(&T) -> usize,
) -> Vec<Vec<T>> {
    let mut result = Vec::new();
    let mut chunk = Vec::new();
    let mut budget = SizeBudget::new(limit);

    for item in items {
        let cost = size_of(&item);

        if budget.needs_flush(cost) {
            result.push(std::mem::take(&mut chunk));
            budget.reset();
        }

        budget.add(cost);
        chunk.push(item);
    }

    if !chunk.is_empty() {
        result.push(chunk);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_yields_no_chunks() {
        let chunks = split_into_sized_chunks(Vec::<&str>::new(), 10, |it| it.len());
        assert!(chunks.is_empty());
    }

    #[test]
    fn everything_fitting_stays_one_chunk() {
        let chunks = split_into_sized_chunks(vec!["a", "b", "c"], 10, |it| it.len());
        assert_eq!(vec![vec!["a", "b", "c"]], chunks);
    }

    #[test]
    fn exactly_on_the_limit_still_fits() {
        let chunks = split_into_sized_chunks(vec!["aaa", "bb"], 5, |it| it.len());
        assert_eq!(vec![vec!["aaa", "bb"]], chunks);
    }

    #[test]
    fn one_byte_over_the_limit_splits() {
        let chunks = split_into_sized_chunks(vec!["aaa", "bbb"], 5, |it| it.len());
        assert_eq!(vec![vec!["aaa"], vec!["bbb"]], chunks);
    }

    #[test]
    fn item_larger_than_the_whole_limit_goes_alone() {
        let chunks = split_into_sized_chunks(vec!["a", "enormous", "b"], 3, |it| it.len());
        assert_eq!(vec![vec!["a"], vec!["enormous"], vec!["b"]], chunks);
    }

    #[test]
    fn zero_cost_items_never_force_a_flush() {
        let chunks = split_into_sized_chunks(vec!["", "", ""], 0, |it| it.len());
        assert_eq!(vec![vec!["", "", ""]], chunks);
    }

    #[test]
    fn budget_admits_an_oversized_item_on_an_empty_batch() {
        let budget = SizeBudget::new(4);
        assert!(!budget.needs_flush(9000));
    }

    #[test]
    fn budget_reports_usage_and_resets() {
        let mut budget = SizeBudget::new(10);

        assert!(budget.is_empty());
        budget.add(4);
        assert_eq!(4, budget.used());
        assert!(!budget.is_empty());
        assert!(!budget.needs_flush(6));
        assert!(budget.needs_flush(7));

        budget.reset();
        assert!(budget.is_empty());
        assert_eq!(10, budget.limit());
    }

    #[test]
    fn budget_does_not_overflow_on_absurd_costs() {
        let mut budget = SizeBudget::new(usize::MAX);
        budget.add(usize::MAX);
        budget.add(usize::MAX);
        assert_eq!(usize::MAX, budget.used());
    }

    /// Two collections sharing one batch — the shape `split_into_sized_chunks`
    /// deliberately does not cover, kept here so the documented alternative is
    /// known to work.
    #[test]
    fn budget_packs_two_collections_into_shared_batches() {
        let numbers = [10usize, 20, 30];
        let words = ["aaaa", "bbbb"];

        let mut budget = SizeBudget::new(6);
        let mut batches: Vec<(Vec<usize>, Vec<&str>)> = Vec::new();
        let mut batch = (Vec::new(), Vec::new());

        for number in numbers {
            if budget.needs_flush(2) {
                batches.push(std::mem::take(&mut batch));
                budget.reset();
            }
            budget.add(2);
            batch.0.push(number);
        }

        for word in words {
            if budget.needs_flush(word.len()) {
                batches.push(std::mem::take(&mut batch));
                budget.reset();
            }
            budget.add(word.len());
            batch.1.push(word);
        }

        batches.push(batch);

        assert_eq!(
            vec![
                (vec![10, 20, 30], vec![]),
                (vec![], vec!["aaaa"]),
                (vec![], vec!["bbbb"]),
            ],
            batches
        );
    }
}
