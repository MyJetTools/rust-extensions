use std::collections::VecDeque;

#[derive(Debug)]
pub struct VecDequeAutoShrink<T> {
    inner: VecDeque<T>,
    auto_shrink_capacity: usize,
}

impl<T> VecDequeAutoShrink<T> {
    pub fn new(auto_shrink_capacity: usize) -> Self {
        Self {
            inner: VecDeque::new(),
            auto_shrink_capacity,
        }
    }

    pub fn new_with_element(auto_shrink_capacity: usize, element: T) -> Self {
        let mut result = Self {
            inner: VecDeque::new(),
            auto_shrink_capacity,
        };

        result.push_back(element);

        result
    }
    fn gc(&mut self) {
        if self.inner.len() < self.auto_shrink_capacity {
            self.inner.shrink_to(self.auto_shrink_capacity);
        }
    }

    pub fn push_back(&mut self, value: T) {
        self.inner.push_back(value);
    }

    pub fn push_front(&mut self, value: T) {
        self.inner.push_front(value);
    }

    pub fn pop_front(&mut self) -> Option<T> {
        let result = self.inner.pop_front();
        self.gc();

        result
    }

    pub fn pop_back(&mut self) -> Option<T> {
        let result = self.inner.pop_back();
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

    pub fn remove(&mut self, index: usize) -> Option<T> {
        let result = self.inner.remove(index);
        self.gc();
        result
    }

    pub fn inter(&self) -> std::collections::vec_deque::Iter<'_, T> {
        self.inner.iter()
    }

    pub fn iter_mut(&mut self) -> std::collections::vec_deque::IterMut<'_, T> {
        self.inner.iter_mut()
    }

    pub fn retain(&mut self, f: impl FnMut(&T) -> bool) {
        self.inner.retain(f);
        self.gc();
    }
}
