pub trait EntityWithKey<TKey: Ord> {
    fn get_key(&self) -> &TKey;
}
