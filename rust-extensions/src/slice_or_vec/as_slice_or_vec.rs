pub enum SliceOrVec<'s, T: Clone> {
    AsSlice(&'s [T]),
    AsVec(Vec<T>),
}

impl<'s, T: Clone> SliceOrVec<'s, T> {
    pub fn create_as_slice(slice: &'s [T]) -> SliceOrVec<'s, T> {
        SliceOrVec::AsSlice(slice)
    }

    pub fn create_as_vec(vec: Vec<T>) -> SliceOrVec<'s, T> {
        SliceOrVec::AsVec(vec)
    }
    pub fn as_slice(&self) -> &[T] {
        match self {
            SliceOrVec::AsSlice(slice) => slice,
            SliceOrVec::AsVec(vec) => vec.as_slice(),
        }
    }

    pub fn is_slice(&self) -> bool {
        match self {
            SliceOrVec::AsSlice(_) => true,
            SliceOrVec::AsVec(_) => false,
        }
    }

    pub fn is_vec(&self) -> bool {
        match self {
            SliceOrVec::AsSlice(_) => false,
            SliceOrVec::AsVec(_) => true,
        }
    }

    pub fn into_vec(self) -> Vec<T> {
        match self {
            SliceOrVec::AsSlice(slice) => slice.to_vec(),
            SliceOrVec::AsVec(vec) => vec,
        }
    }

    pub fn get_len(&self) -> usize {
        match self {
            SliceOrVec::AsSlice(slice) => slice.len(),
            SliceOrVec::AsVec(vec) => vec.len(),
        }
    }
}

impl<'s, T: Clone> std::fmt::Debug for SliceOrVec<'s, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SliceOrVec::AsSlice(items) => f
                .debug_struct("SliceOrVec")
                .field("SliceLen", &items.len())
                .finish(),
            SliceOrVec::AsVec(items) => f
                .debug_struct("SliceOrVec")
                .field("VecLen", &items.len())
                .finish(),
        }
    }
}

impl<'s, T: Clone> Into<SliceOrVec<'s, T>> for Vec<T> {
    fn into(self) -> SliceOrVec<'s, T> {
        SliceOrVec::AsVec(self)
    }
}

impl<'s, T: Clone> Into<SliceOrVec<'s, T>> for &'s Vec<T> {
    fn into(self) -> SliceOrVec<'s, T> {
        SliceOrVec::AsSlice(self.as_slice())
    }
}

impl<'s, T: Clone> Into<SliceOrVec<'s, T>> for &'s [T] {
    fn into(self) -> SliceOrVec<'s, T> {
        SliceOrVec::AsSlice(self)
    }
}

impl<'s> Into<SliceOrVec<'s, u8>> for String {
    fn into(self) -> SliceOrVec<'s, u8> {
        SliceOrVec::AsVec(self.into_bytes())
    }
}

impl<'s> Into<SliceOrVec<'s, u8>> for &'s str {
    fn into(self) -> SliceOrVec<'s, u8> {
        SliceOrVec::AsSlice(self.as_bytes())
    }
}

impl<'s> Into<SliceOrVec<'s, u8>> for &'s String {
    fn into(self) -> SliceOrVec<'s, u8> {
        SliceOrVec::AsSlice(self.as_bytes())
    }
}
