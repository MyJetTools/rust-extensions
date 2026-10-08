/// A source of bytes which is read in chunks - a file, a response body, a blob
/// downloaded part by part.
///
/// `get_next()` returns `Ok(Some(chunk))` with the next chunk and `Ok(None)`
/// once there is nothing left to read.
#[async_trait::async_trait]
pub trait AsyncBytesReader<TError> {
    async fn get_next(&self) -> Result<Option<Vec<u8>>, TError>;

    /// The size of the whole stream in bytes - `None` if it is not known until
    /// the stream is read to the end.
    fn get_size(&self) -> Option<usize>;

    /// Reads the stream to the end and returns everything as a single `Vec`.
    ///
    /// A known size - see `get_size()` - is allocated at once, so the `Vec`
    /// does not grow while the chunks are being appended.
    async fn into_vec(&self) -> Result<Vec<u8>, TError> {
        let mut result = match self.get_size() {
            Some(size) => Vec::with_capacity(size),
            None => Vec::new(),
        };

        while let Some(chunk) = self.get_next().await? {
            result.extend_from_slice(&chunk);
        }

        Ok(result)
    }
}

#[cfg(all(test, feature = "with-tokio"))]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::AsyncBytesReader;

    struct Chunks {
        chunks: Vec<Result<Vec<u8>, String>>,
        next_chunk: AtomicUsize,
        size: Option<usize>,
    }

    impl Chunks {
        fn new(chunks: Vec<Result<Vec<u8>, String>>) -> Self {
            Self {
                chunks,
                next_chunk: AtomicUsize::new(0),
                size: None,
            }
        }

        fn with_size(mut self, size: usize) -> Self {
            self.size = Some(size);
            self
        }
    }

    #[async_trait::async_trait]
    impl AsyncBytesReader<String> for Chunks {
        async fn get_next(&self) -> Result<Option<Vec<u8>>, String> {
            let chunk_no = self.next_chunk.fetch_add(1, Ordering::Relaxed);
            self.chunks.get(chunk_no).cloned().transpose()
        }

        fn get_size(&self) -> Option<usize> {
            self.size
        }
    }

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
    }

    #[test]
    fn reads_chunks_until_none() {
        rt().block_on(async {
            let src: Arc<dyn AsyncBytesReader<String> + Send + Sync + 'static> =
                Arc::new(Chunks::new(vec![Ok(vec![1, 2]), Ok(vec![3])]));

            // Read from a spawned task - the future of `get_next()` is `Send`.
            let result = tokio::spawn(async move {
                let mut result = Vec::new();
                while let Some(chunk) = src.get_next().await.unwrap() {
                    result.extend(chunk);
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
            let src = Chunks::new(vec![Ok(vec![1]), Err("no connection".to_string())]);

            assert_eq!(src.get_next().await, Ok(Some(vec![1])));
            assert_eq!(src.get_next().await, Err("no connection".to_string()));
            assert_eq!(src.get_next().await, Ok(None));
        });
    }

    #[test]
    fn into_vec_merges_all_the_chunks() {
        rt().block_on(async {
            let src: Arc<dyn AsyncBytesReader<String> + Send + Sync + 'static> =
                Arc::new(Chunks::new(vec![Ok(vec![1, 2]), Ok(vec![3])]));

            let result = tokio::spawn(async move { src.into_vec().await })
                .await
                .unwrap();

            assert_eq!(result, Ok(vec![1, 2, 3]));
        });
    }

    #[test]
    fn into_vec_allocates_a_known_size_at_once() {
        rt().block_on(async {
            // Three bytes would never make a `Vec` grow that far by itself.
            let src = Chunks::new(vec![Ok(vec![1, 2]), Ok(vec![3])]).with_size(100);

            let result = src.into_vec().await.unwrap();

            assert_eq!(result, vec![1, 2, 3]);
            assert!(result.capacity() >= 100);
        });
    }

    #[test]
    fn into_vec_stops_at_the_first_error() {
        rt().block_on(async {
            let src = Chunks::new(vec![
                Ok(vec![1]),
                Err("no connection".to_string()),
                Ok(vec![2]),
            ]);

            assert_eq!(src.into_vec().await, Err("no connection".to_string()));
            // The chunk after the broken one was not asked for.
            assert_eq!(src.get_next().await, Ok(Some(vec![2])));
        });
    }
}
