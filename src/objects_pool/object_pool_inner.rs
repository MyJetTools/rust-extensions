use std::sync::Arc;

use tokio::sync::Mutex;

pub enum RentResult<T: Sync + Send + 'static> {
    Rented(T),
    CreateNew,
    Wait,
}

pub struct ObjectPoolInner<T: Sync + Send + 'static> {
    pool: Vec<Arc<Mutex<T>>>,
    created_amount: usize,
}

impl<T: Sync + Send + 'static> ObjectPoolInner<T> {
    pub fn new() -> Self {
        Self {
            pool: Vec::new(),
            created_amount: 0,
        }
    }

    pub fn take(&mut self, max_amount: usize) -> RentResult<Arc<Mutex<T>>> {
        if self.pool.len() > 0 {
            let result = self.pool.pop().unwrap();
            return RentResult::Rented(result);
        }

        if self.created_amount >= max_amount {
            return RentResult::Wait;
        }

        self.created_amount += 1;

        return RentResult::CreateNew;
    }

    pub fn return_element(&mut self, item: Arc<Mutex<T>>) {
        self.pool.push(item);
    }
}
