use std::sync::Arc;

use parking_lot::Mutex;

use super::{ObjectPoolInner, RentedObject};

use super::object_pool_inner::RentResult;

#[async_trait::async_trait]
pub trait ObjectsPoolFactory<T: Sync + Send + 'static> {
    async fn create_new(&self) -> T;
}

pub struct ObjectsPool<T: Sync + Send + 'static, TFactory: ObjectsPoolFactory<T>> {
    inner: Arc<Mutex<ObjectPoolInner<T>>>,

    max_pool_size: usize,
    factory: Arc<TFactory>,
}

impl<T: Sync + Send + 'static, TFactory: ObjectsPoolFactory<T>> ObjectsPool<T, TFactory> {
    pub fn new(max_pool_size: usize, factory: Arc<TFactory>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ObjectPoolInner::new())),
            max_pool_size,
            factory,
        }
    }

    pub async fn get_element(&self) -> RentedObject<T> {
        loop {
            let rent_result = {
                let mut write_access = self.inner.lock();
                write_access.take(self.max_pool_size)
            };

            match rent_result {
                RentResult::Rented(result) => {
                    return RentedObject::new(self.inner.clone(), result);
                }
                RentResult::CreateNew => {
                    return RentedObject::new(self.inner.clone(), self.factory.create_new().await);
                }
                RentResult::Wait => {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
        }
    }
}
