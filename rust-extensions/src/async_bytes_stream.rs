use std::ops::Deref;
use std::sync::Arc;

/// A stream of bytes which is read in chunks - a file, a response body, a blob
/// downloaded part by part.
///
/// `get_next()` returns `Ok(Some(chunk))` with the next chunk and `Ok(None)`
/// once there is nothing left to read.
///
/// A chunk gives its bytes as a `&[u8]` through `Deref`, whatever holds them: a
/// `Vec<u8>`, a `Bytes`, a buffer of a [`crate::DoubleBuffer`]. Dropping a chunk
/// says that it is processed - a stream which reads into buffers of its own reads
/// into that one again. So a chunk is processed and dropped, and what is needed
/// later is copied out of it.
#[async_trait::async_trait]
pub trait AsyncBytesStream<TError> {
    /// What holds the bytes of a chunk
    type Chunk: Deref<Target = [u8]> + Send;

    async fn get_next(&self) -> Result<Option<Self::Chunk>, TError>;

    /// The size of the whole stream in bytes - `None` if it is not known until
    /// the stream is read to the end.
    fn get_size(&self) -> Option<usize>;

    /// Reads the stream to the end and returns everything as a single `Vec`.
    ///
    /// Every chunk is copied into it. With the size of the stream known, the `Vec`
    /// is allocated for all of it at once.
    async fn into_vec(&self) -> Result<Vec<u8>, TError> {
        let mut result = Vec::with_capacity(self.get_size().unwrap_or(0));

        while let Some(chunk) = self.get_next().await? {
            result.extend_from_slice(&chunk);
        }

        Ok(result)
    }
}

/// A stream behind an `Arc` is a stream as well - so what takes one takes an
/// `Arc<dyn AsyncBytesStream<TError, Chunk = ...> + Send + Sync>` too.
#[async_trait::async_trait]
impl<TError, TStream> AsyncBytesStream<TError> for Arc<TStream>
where
    TStream: AsyncBytesStream<TError> + Send + Sync + ?Sized,
{
    type Chunk = TStream::Chunk;

    async fn get_next(&self) -> Result<Option<Self::Chunk>, TError> {
        self.as_ref().get_next().await
    }

    fn get_size(&self) -> Option<usize> {
        self.as_ref().get_size()
    }

    async fn into_vec(&self) -> Result<Vec<u8>, TError> {
        self.as_ref().into_vec().await
    }
}

#[cfg(all(test, feature = "with-tokio"))]
mod tests {
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use parking_lot::Mutex;

    use super::AsyncBytesStream;

    /// Hands its chunks over one by one, and counts how many times it is asked
    struct Chunks {
        chunks: Mutex<VecDeque<Result<Vec<u8>, String>>>,
        size: Option<usize>,
        requests: AtomicUsize,
    }

    impl Chunks {
        fn new(chunks: Vec<Result<Vec<u8>, String>>) -> Self {
            Self {
                chunks: Mutex::new(chunks.into()),
                size: None,
                requests: AtomicUsize::new(0),
            }
        }

        fn with_size(mut self, size: usize) -> Self {
            self.size = Some(size);
            self
        }

        fn requests(&self) -> usize {
            self.requests.load(Ordering::Relaxed)
        }
    }

    #[async_trait::async_trait]
    impl AsyncBytesStream<String> for Chunks {
        type Chunk = Vec<u8>;

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
            let src: Arc<dyn AsyncBytesStream<String, Chunk = Vec<u8>> + Send + Sync> =
                Arc::new(Chunks::new(vec![Ok(vec![1, 2]), Ok(vec![3])]));

            // Read from a spawned task - the future of `get_next()` is `Send`.
            let result = tokio::spawn(async move {
                let mut result = Vec::new();
                while let Some(chunk) = src.get_next().await.unwrap() {
                    result.extend_from_slice(&chunk);
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
            let src: Arc<dyn AsyncBytesStream<String, Chunk = Vec<u8>> + Send + Sync> =
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
    fn into_vec_allocates_the_known_size_at_once() {
        rt().block_on(async {
            // Three bytes would never make a `Vec` grow that far by itself.
            let src = Chunks::new(vec![Ok(vec![1, 2]), Ok(vec![3])]).with_size(100);

            let result = src.into_vec().await.unwrap();

            assert_eq!(result, vec![1, 2, 3]);
            assert!(result.capacity() >= 100);
        });
    }

    #[test]
    fn into_vec_reads_to_the_end_whatever_the_size_says() {
        rt().block_on(async {
            let src = Chunks::new(vec![Ok(vec![1, 2, 3])]).with_size(3);

            assert_eq!(src.into_vec().await, Ok(vec![1, 2, 3]));
            // The size is not taken for the end - the stream says where it is.
            assert_eq!(src.requests(), 2);

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

            assert_eq!(src.into_vec().await, Err("no connection".to_string()));
            // The chunk after the broken one was not asked for.
            assert_eq!(src.requests(), 2);
        });
    }

    #[test]
    fn into_vec_returns_the_error_of_the_first_chunk() {
        rt().block_on(async {
            let src = Chunks::new(vec![Err("no connection".to_string())]);

            assert_eq!(src.into_vec().await, Err("no connection".to_string()));
        });
    }

    #[test]
    fn a_stream_behind_an_arc_is_a_stream() {
        async fn read_all<TStream: AsyncBytesStream<String> + Sync>(src: TStream) -> Vec<u8> {
            src.into_vec().await.unwrap()
        }

        rt().block_on(async {
            let src = Arc::new(Chunks::new(vec![Ok(vec![1, 2]), Ok(vec![3])]).with_size(3));
            assert_eq!(src.get_size(), Some(3));
            assert_eq!(read_all(src).await, vec![1, 2, 3]);

            let src: Arc<dyn AsyncBytesStream<String, Chunk = Vec<u8>> + Send + Sync> =
                Arc::new(Chunks::new(vec![Ok(vec![1, 2]), Ok(vec![3])]));
            assert_eq!(read_all(src).await, vec![1, 2, 3]);
        });
    }
}
