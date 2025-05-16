use tokio::sync::Mutex;

#[derive(Default)]
pub struct QueueToSave<T> {
    queue: Mutex<Vec<T>>,
}

impl<T> QueueToSave<T> {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(vec![]),
        }
    }
    pub async fn enqueue(&self, items: impl Iterator<Item = T>) {
        let mut write_access = self.queue.lock().await;
        write_access.extend(items);
    }

    pub async fn dequeue(&self, max_amount: usize) -> Option<Vec<T>> {
        let mut write_access = self.queue.lock().await;

        if write_access.len() == 0 {
            return None;
        }

        if write_access.len() <= max_amount {
            return Some(std::mem::take(&mut *write_access));
        }

        let mut result = Vec::with_capacity(max_amount);

        while result.len() < max_amount {
            if let Some(item) = write_access.pop() {
                result.push(item);
            } else {
                break;
            }
        }

        Some(result)
    }
}
