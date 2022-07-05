use std::sync::Arc;

use tokio::sync::Mutex;

use super::ObjectPoolInner;

pub struct RentedObject<T: Sync + Send + 'static> {
    pub value: Arc<Mutex<T>>,
    inner: Arc<Mutex<ObjectPoolInner<T>>>,
}

impl<T: Sync + Send + 'static> RentedObject<T> {
    pub fn new(inner: Arc<Mutex<ObjectPoolInner<T>>>, value: Arc<Mutex<T>>) -> Self {
        Self { value, inner }
    }
}

impl<T: Sync + Send + 'static> Drop for RentedObject<T> {
    fn drop(&mut self) {
        let inner = self.inner.clone();
        let value = self.value.clone();
        tokio::spawn(async move {
            let mut inner = inner.lock().await;
            inner.return_element(value);
        });
    }
}
