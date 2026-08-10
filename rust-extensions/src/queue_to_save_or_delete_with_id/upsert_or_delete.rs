use super::PersistObjectId;

/// The state which is flushed to the handler for a certain ID.
///
/// `Upsert` carries the object itself, `Delete` carries only the ID - the object
/// is dropped at the moment `enqueue_delete` is called, since there is nothing to save anymore.
pub enum UpsertOrDelete<ID, T> {
    Upsert(T),
    Delete(ID),
}

impl<ID, T> UpsertOrDelete<ID, T> {
    pub fn is_upsert(&self) -> bool {
        matches!(self, Self::Upsert(_))
    }

    pub fn is_delete(&self) -> bool {
        matches!(self, Self::Delete(_))
    }

    pub fn as_upsert(&self) -> Option<&T> {
        match self {
            Self::Upsert(value) => Some(value),
            Self::Delete(_) => None,
        }
    }

    pub fn as_delete(&self) -> Option<&ID> {
        match self {
            Self::Upsert(_) => None,
            Self::Delete(id) => Some(id),
        }
    }

    pub fn unwrap_as_upsert(&self) -> &T {
        match self {
            Self::Upsert(value) => value,
            Self::Delete(_) => panic!("UpsertOrDelete is Delete but Upsert is requested"),
        }
    }

    pub fn unwrap_as_delete(&self) -> &ID {
        match self {
            Self::Upsert(_) => panic!("UpsertOrDelete is Upsert but Delete is requested"),
            Self::Delete(id) => id,
        }
    }

    /// Splits the chunk into the two bulk operations a storage normally speaks:
    /// objects to insert-or-replace and IDs to delete.
    ///
    /// The two never intersect - a single ID is either in one state or in the other.
    pub fn split(items: Vec<Self>) -> (Vec<T>, Vec<ID>) {
        let mut to_upsert = Vec::with_capacity(items.len());
        let mut to_delete = Vec::new();

        for item in items {
            match item {
                Self::Upsert(value) => to_upsert.push(value),
                Self::Delete(id) => to_delete.push(id),
            }
        }

        (to_upsert, to_delete)
    }
}

impl<ID: std::hash::Hash + Eq + Clone, T: PersistObjectId<ID>> UpsertOrDelete<ID, T> {
    pub fn get_id(&self) -> &ID {
        match self {
            Self::Upsert(value) => value.get_persist_object_id(),
            Self::Delete(id) => id,
        }
    }
}
