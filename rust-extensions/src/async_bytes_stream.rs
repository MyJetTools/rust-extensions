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
    /// The first chunk becomes the result itself, so nothing is allocated and
    /// nothing is copied for it:
    /// - a chunk of `get_size()` bytes is the whole stream - it is returned as
    ///   it is, and the stream is not asked for more;
    /// - a smaller chunk of a stream with a known size is extended up to that
    ///   size at once, so it does not grow while the rest is being appended;
    /// - with no size known the rest is just appended to it.
    ///
    /// `get_size()` is asked once the first chunk has arrived - a size which
    /// becomes known together with the first chunk is taken into account.
    async fn into_vec(&self) -> Result<Vec<u8>, TError> {
        let Some(mut result) = self.get_next().await? else {
            return Ok(Vec::new());
        };

        if let Some(size) = self.get_size() {
            if result.len() == size {
                return Ok(result);
            }

            if result.len() < size {
                result.reserve_exact(size - result.len());
            }
        }

        while let Some(chunk) = self.get_next().await? {
            result.extend_from_slice(&chunk);
        }

        Ok(result)
    }
}

#[cfg(all(test, feature = "with-tokio"))]
mod tests {
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use parking_lot::Mutex;

    use super::AsyncBytesReader;

    /// Hands the chunks over by ownership - the very same buffers, so it is
    /// seen whether the reader of the stream has copied them.
    struct Chunks {
        chunks: Mutex<VecDeque<Result<Vec<u8>, String>>>,
        size: Option<usize>,
        requests: Arc<AtomicUsize>,
    }

    impl Chunks {
        fn new(chunks: Vec<Result<Vec<u8>, String>>) -> Self {
            Self {
                chunks: Mutex::new(chunks.into()),
                size: None,
                requests: Arc::new(AtomicUsize::new(0)),
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
            self.requests.fetch_add(1, Ordering::Relaxed);
            self.chunks.lock().pop_front().transpose()
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
                Arc::new(Chunks::new(vec![
                    Ok(vec![1, 2]),
                    Ok(vec![3]),
                    Ok(vec![4, 5]),
                ]));

            // The future of `into_vec()` is `Send` as well.
            let result = tokio::spawn(async move { src.into_vec().await })
                .await
                .unwrap();

            assert_eq!(result, Ok(vec![1, 2, 3, 4, 5]));
        });
    }

    #[test]
    fn into_vec_returns_the_chunk_of_the_whole_size_as_it_is() {
        rt().block_on(async {
            let chunk = vec![1, 2, 3];
            let chunk_ptr = chunk.as_ptr();

            let src = Chunks::new(vec![Ok(chunk)]).with_size(3);
            let requests = src.requests.clone();

            let result = src.into_vec().await.unwrap();

            assert_eq!(result, vec![1, 2, 3]);
            // The very same buffer - and the stream was not asked for more.
            assert_eq!(result.as_ptr(), chunk_ptr);
            assert_eq!(requests.load(Ordering::Relaxed), 1);
        });
    }

    #[test]
    fn into_vec_extends_the_first_chunk_up_to_a_known_size() {
        rt().block_on(async {
            // Three bytes would never make a `Vec` grow that far by itself.
            let src = Chunks::new(vec![Ok(vec![1, 2]), Ok(vec![3])]).with_size(100);

            let result = src.into_vec().await.unwrap();

            assert_eq!(result, vec![1, 2, 3]);
            assert!(result.capacity() >= 100);
        });
    }

    #[test]
    fn into_vec_with_no_size_known_takes_the_first_chunk_as_the_result() {
        rt().block_on(async {
            let chunk = vec![1, 2, 3];
            let chunk_ptr = chunk.as_ptr();

            let src = Chunks::new(vec![Ok(chunk)]);
            let requests = src.requests.clone();

            let result = src.into_vec().await.unwrap();

            assert_eq!(result, vec![1, 2, 3]);
            assert_eq!(result.as_ptr(), chunk_ptr);
            // Nobody knows that it was the last one - until the stream says so.
            assert_eq!(requests.load(Ordering::Relaxed), 2);
        });
    }

    #[test]
    fn into_vec_reads_to_the_end_if_the_first_chunk_is_bigger_than_the_size() {
        rt().block_on(async {
            let src = Chunks::new(vec![Ok(vec![1, 2, 3]), Ok(vec![4])]).with_size(2);

            assert_eq!(src.into_vec().await, Ok(vec![1, 2, 3, 4]));
        });
    }

    #[test]
    fn into_vec_of_an_empty_stream_is_empty() {
        rt().block_on(async {
            assert_eq!(Chunks::new(vec![]).into_vec().await, Ok(vec![]));
            assert_eq!(
                Chunks::new(vec![]).with_size(10).into_vec().await,
                Ok(vec![])
            );
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
            let requests = src.requests.clone();

            assert_eq!(src.into_vec().await, Err("no connection".to_string()));
            // The chunk after the broken one was not asked for.
            assert_eq!(requests.load(Ordering::Relaxed), 2);
        });
    }

    #[test]
    fn into_vec_returns_the_error_of_the_first_chunk() {
        rt().block_on(async {
            let src = Chunks::new(vec![Err("no connection".to_string())]);

            assert_eq!(src.into_vec().await, Err("no connection".to_string()));
        });
    }
}
