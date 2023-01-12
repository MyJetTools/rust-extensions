use std::sync::Arc;

use tokio::sync::Mutex;

use super::ObjectPoolInner;

pub struct RentedObject<T: Sync + Send + 'static> {
    value: Option<T>,
    inner: Arc<Mutex<ObjectPoolInner<T>>>,
}

impl<T: Sync + Send + 'static> RentedObject<T> {
    pub fn new(inner: Arc<Mutex<ObjectPoolInner<T>>>, value: T) -> Self {
        Self {
            value: Some(value),
            inner,
        }
    }

    pub fn get_value(&self) -> &T {
        self.value.as_ref().expect("Somehow value went to None")
    }
}

impl<T: Sync + Send + 'static> Drop for RentedObject<T> {
    fn drop(&mut self) {
        let inner = self.inner.clone();
        let value = self.value.take().unwrap();
        tokio::spawn(async move {
            let mut inner = inner.lock().await;
            inner.return_element(value);
        });
    }
}

impl<T: Sync + Send + 'static> AsRef<T> for RentedObject<T> {
    fn as_ref(&self) -> &T {
        self.get_value()
    }
}
