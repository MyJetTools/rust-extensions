pub trait EntityWithKey<TKey: Ord> {
    fn get_key(&self) -> &TKey;
}

pub trait EntityWithStrKey {
    fn get_key(&self) -> &str;
}
