pub trait EntityWith2StrKey {
    fn get_primary_key(&self) -> &str;
    fn get_secondary_key(&self) -> &str;
}
