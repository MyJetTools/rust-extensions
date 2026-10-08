/// A source which is read in portions - a paged request, a cursor, a stream of
/// batches.
///
/// `get_next()` returns `Ok(Some(items))` with the next portion and `Ok(None)`
/// once there is nothing left to read.
#[async_trait::async_trait]
pub trait AsyncIterator<T, TError> {
    async fn get_next(&self) -> Result<Option<Vec<T>>, TError>;
}

#[cfg(all(test, feature = "with-tokio"))]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::AsyncIterator;

    struct Pages {
        pages: Vec<Result<Vec<u32>, String>>,
        next_page: AtomicUsize,
    }

    impl Pages {
        fn new(pages: Vec<Result<Vec<u32>, String>>) -> Self {
            Self {
                pages,
                next_page: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl AsyncIterator<u32, String> for Pages {
        async fn get_next(&self) -> Result<Option<Vec<u32>>, String> {
            let page_no = self.next_page.fetch_add(1, Ordering::Relaxed);
            self.pages.get(page_no).cloned().transpose()
        }
    }

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
    }

    #[test]
    fn reads_portions_until_none() {
        rt().block_on(async {
            let src: Arc<dyn AsyncIterator<u32, String> + Send + Sync + 'static> =
                Arc::new(Pages::new(vec![Ok(vec![1, 2]), Ok(vec![3])]));

            // Read from a spawned task - the future of `get_next()` is `Send`.
            let result = tokio::spawn(async move {
                let mut result = Vec::new();
                while let Some(items) = src.get_next().await.unwrap() {
                    result.extend(items);
                }
                result
            })
            .await
            .unwrap();

            assert_eq!(result, vec![1, 2, 3]);
        });
    }

    #[test]
    fn error_is_handed_to_the_caller() {
        rt().block_on(async {
            let src = Pages::new(vec![Ok(vec![1]), Err("no connection".to_string())]);

            assert_eq!(src.get_next().await, Ok(Some(vec![1])));
            assert_eq!(src.get_next().await, Err("no connection".to_string()));
            assert_eq!(src.get_next().await, Ok(None));
        });
    }
}
