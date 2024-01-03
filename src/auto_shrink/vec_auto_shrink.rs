pub struct VecAutoShrink<T> {
    inner: Vec<T>,
    auto_shrink_capacity: usize,
}

impl<T> VecAutoShrink<T> {
    pub fn new(auto_shrink_capacity: usize) -> Self {
        Self {
            inner: Vec::new(),
            auto_shrink_capacity,
        }
    }

    pub fn new_with_element(auto_shrink_capacity: usize, element: T) -> Self {
        let mut result = Self {
            inner: Vec::new(),
            auto_shrink_capacity,
        };

        result.push(element);

        result
    }

    fn gc(&mut self) {
        if self.inner.len() < self.auto_shrink_capacity {
            self.inner.shrink_to(self.auto_shrink_capacity);
        }
    }

    pub fn push(&mut self, value: T) {
        self.inner.push(value);
    }

    pub fn pop(&mut self) -> Option<T> {
        let result = self.inner.pop();
        self.gc();

        result
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
        self.gc();
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.inner.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.inner.get_mut(index)
    }

    pub fn remove(&mut self, index: usize) -> T {
        let result = self.inner.remove(index);
        self.gc();
        result
    }

    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    pub fn iter(&self) -> std::slice::Iter<T> {
        self.inner.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<T> {
        self.inner.iter_mut()
    }

    pub fn retain(&mut self, f: impl FnMut(&T) -> bool) {
        self.inner.retain(f);
        self.gc();
    }
}
