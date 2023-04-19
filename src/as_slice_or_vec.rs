pub enum AsSliceOrVec<'s, T: Clone> {
    AsSlice(&'s [T]),
    AsVec(Vec<T>),
}

impl<'s, T: Clone> AsSliceOrVec<'s, T> {
    pub fn as_slice(&self) -> &[T] {
        match self {
            AsSliceOrVec::AsSlice(slice) => slice,
            AsSliceOrVec::AsVec(vec) => vec.as_slice(),
        }
    }

    pub fn into_vec(self) -> Vec<T> {
        match self {
            AsSliceOrVec::AsSlice(slice) => slice.to_vec(),
            AsSliceOrVec::AsVec(vec) => vec,
        }
    }
}

impl<'s, T: Clone> Into<AsSliceOrVec<'s, T>> for Vec<T> {
    fn into(self) -> AsSliceOrVec<'s, T> {
        AsSliceOrVec::AsVec(self)
    }
}

impl<'s, T: Clone> Into<AsSliceOrVec<'s, T>> for &'s Vec<T> {
    fn into(self) -> AsSliceOrVec<'s, T> {
        AsSliceOrVec::AsSlice(self.as_slice())
    }
}

impl<'s, T: Clone> Into<AsSliceOrVec<'s, T>> for &'s [T] {
    fn into(self) -> AsSliceOrVec<'s, T> {
        AsSliceOrVec::AsSlice(self)
    }
}

impl<'s> Into<AsSliceOrVec<'s, u8>> for String {
    fn into(self) -> AsSliceOrVec<'s, u8> {
        AsSliceOrVec::AsVec(self.into_bytes())
    }
}

impl<'s> Into<AsSliceOrVec<'s, u8>> for &'s str {
    fn into(self) -> AsSliceOrVec<'s, u8> {
        AsSliceOrVec::AsSlice(self.as_bytes())
    }
}

impl<'s> Into<AsSliceOrVec<'s, u8>> for &'s String {
    fn into(self) -> AsSliceOrVec<'s, u8> {
        AsSliceOrVec::AsSlice(self.as_bytes())
    }
}
