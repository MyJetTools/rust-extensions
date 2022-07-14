pub struct LazyVec<T> {
    pub result: Option<Vec<T>>,
}

impl<T> LazyVec<T> {
    pub fn new() -> Self {
        Self { result: None }
    }

    pub fn add(&mut self, t: T) {
        if self.result.is_none() {
            self.result = Some(Vec::new());
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
