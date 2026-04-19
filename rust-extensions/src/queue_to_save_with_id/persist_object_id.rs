pub trait PersistObjectId<ID: std::hash::Hash + Eq + Clone> {
    fn get_persist_object_id(&self) -> &ID;
}
