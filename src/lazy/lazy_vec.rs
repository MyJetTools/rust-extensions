pub struct LazyVec<T> {
    pub result: Option<Vec<T>>,
    pub capacity: Option<usize>,
}

impl<T> LazyVec<T> {
    pub fn new() -> Self {
        Self {
            result: None,
            capacity: None,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            result: None,
            capacity: Some(capacity),
        }
    }

    pub fn add(&mut self, t: T) {
        if self.result.is_none() {
            if let Some(capacity) = self.capacity {
                self.result = Some(Vec::with_capacity(capacity));
            } else {
                self.result = Some(Vec::new());
            }
        }

        self.result.as_mut().unwrap().push(t);
    }

    pub fn get_result(self) -> Option<Vec<T>> {
        self.result
    }

    pub fn is_empty(&self) -> bool {
        self.result.is_none()
    }
}
