use bytes::{Buf, Bytes, BytesMut};

use crate::AsyncBytesStream;

/// Reads an [`AsyncBytesStream`] for the one who parses it - a JSON after a JSON, a
/// line after a line, a frame after a frame.
///
/// A stream is cut into chunks where it happens to be, so what is being parsed may
/// begin in one chunk and end in the next one. The reader keeps what is read and not
/// parsed yet as one run of bytes, whatever the chunks were:
/// - `as_slice()` is what there is to parse;
/// - `mark_as_read()` drops what is parsed from the beginning of it;
/// - `get_next()` reads the next chunk in when there is not enough to parse;
/// - `read_mode()` is `false` once the stream is read to its end.
///
/// The bytes are kept in a `Bytes`, and they are copied only to be joined:
/// - a chunk which comes when nothing is left to parse is taken as it is;
/// - a chunk which comes after bytes which are not parsed yet is put behind them -
///   in place when nobody else holds the buffer, and into a new buffer together
///   with those bytes when somebody does;
/// - `mark_as_read()` moves nothing.
pub struct BufferedReader<TError, TStream: AsyncBytesStream<TError>> {
    stream: TStream,
    buffer: Bytes,
    read_mode: bool,
    // `TError` is what `get_next()` fails with - the reader keeps none of it
    error: std::marker::PhantomData<fn() -> TError>,
}

impl<TError, TStream: AsyncBytesStream<TError>> BufferedReader<TError, TStream> {
    /// Nothing is read until `get_next()` is called.
    pub fn new(stream: TStream) -> Self {
        Self {
            stream,
            buffer: Bytes::new(),
            read_mode: true,
            error: std::marker::PhantomData,
        }
    }

    /// Reads the next chunk of the stream in and returns all there is to parse - the
    /// bytes `as_slice()` gives, the chunk at the end of them. A chunk with nothing
    /// in it is passed by.
    ///
    /// At the end of the stream it returns what is left, which may be nothing, and
    /// `read_mode()` is `false` from then on - the stream is not asked again.
    ///
    /// What is returned is not a copy: it shares the buffer of the reader, and it
    /// stays as it is whatever is read after it. While it is kept, the reader can
    /// not put the next chunk into that buffer, so it copies the bytes which are not
    /// parsed yet to a new one. Drop it before the next call - unless it is there to
    /// be kept.
    ///
    /// An error of the stream is returned as it is. Nothing which is read is lost,
    /// and the next call asks the stream again.
    pub async fn get_next(&mut self) -> Result<Bytes, TError> {
        while self.read_mode {
            match self.stream.get_next().await? {
                Some(chunk) if chunk.is_empty() => {}
                Some(chunk) => {
                    self.append(chunk);
                    break;
                }
                None => self.read_mode = false,
            }
        }

        Ok(self.buffer.clone())
    }

    /// What is read and not marked as read yet.
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
    }

    /// `false` - the stream is read to its end: `get_next()` brings nothing in any
    /// more, and what `as_slice()` gives is all that is left.
    pub fn read_mode(&self) -> bool {
        self.read_mode
    }

    /// Drops `size` bytes from the beginning of what is read - they are parsed. The
    /// bytes which are left stay where they are.
    ///
    /// Panics when there are fewer than `size` bytes.
    pub fn mark_as_read(&mut self, size: usize) {
        assert!(
            size <= self.buffer.len(),
            "mark_as_read: {} bytes are marked as read, and there are {} of them",
            size,
            self.buffer.len()
        );

        self.buffer.advance(size);
    }

    /// Puts the chunk behind the bytes which are read and not parsed yet
    fn append(&mut self, chunk: Bytes) {
        if self.buffer.is_empty() {
            // Nothing to join the chunk with - it is the buffer now, as it has come
            self.buffer = chunk;
            return;
        }

        // The buffer nobody else holds takes the chunk in place: what is parsed gives
        // its room back, and the buffer grows the way a `Vec` does
        let mut joined = match std::mem::take(&mut self.buffer).try_into_mut() {
            Ok(joined) => joined,
            Err(not_parsed) => {
                let mut joined = BytesMut::with_capacity(not_parsed.len() + chunk.len());
                joined.extend_from_slice(&not_parsed);
                joined
            }
        };

        joined.extend_from_slice(&chunk);
        self.buffer = joined.freeze();
    }
}

#[cfg(all(test, feature = "with-tokio"))]
mod tests {
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use bytes::Bytes;
    use parking_lot::Mutex;

    use super::BufferedReader;
    use crate::AsyncBytesStream;

    struct Chunks {
        chunks: Mutex<VecDeque<Result<Bytes, String>>>,
        requests: Arc<AtomicUsize>,
    }

    impl Chunks {
        fn new(chunks: Vec<Result<Bytes, String>>) -> Self {
            Self {
                chunks: Mutex::new(chunks.into()),
                requests: Arc::new(AtomicUsize::new(0)),
            }
        }

        /// Each chunk is made of a `Vec` of its own, so nobody else holds its buffer
        fn of(chunks: &[&str]) -> Self {
            Self::new(
                chunks
                    .iter()
                    .map(|chunk| Ok(Bytes::from(chunk.as_bytes().to_vec())))
                    .collect(),
            )
        }
    }

    #[async_trait::async_trait]
    impl AsyncBytesStream<String> for Chunks {
        async fn get_next(&self) -> Result<Option<Bytes>, String> {
            self.requests.fetch_add(1, Ordering::Relaxed);
            self.chunks.lock().pop_front().transpose()
        }

        fn get_size(&self) -> Option<usize> {
            None
        }
    }

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
    }

    #[test]
    fn as_slice_is_what_is_read_and_not_marked_as_read() {
        rt().block_on(async {
            let mut reader = BufferedReader::new(Chunks::of(&["Hello", "World"]));

            // Nothing is read until it is asked for
            assert!(reader.as_slice().is_empty());
            assert!(reader.read_mode());

            assert_eq!(reader.get_next().await.unwrap(), "Hello");
            assert_eq!(reader.as_slice(), b"Hello");

            reader.mark_as_read(2);
            assert_eq!(reader.as_slice(), b"llo");

            // What is not parsed yet and the next chunk are one run of bytes
            assert_eq!(reader.get_next().await.unwrap(), "lloWorld");
            assert_eq!(reader.as_slice(), b"lloWorld");

            reader.mark_as_read(8);
            assert!(reader.as_slice().is_empty());
        });
    }

    #[test]
    fn the_end_of_the_stream_turns_read_mode_off() {
        rt().block_on(async {
            let src = Chunks::of(&["ab"]);
            let requests = src.requests.clone();
            let mut reader = BufferedReader::new(src);

            assert_eq!(reader.get_next().await.unwrap(), "ab");
            // Nobody knows that it was the last one - until the stream says so
            assert!(reader.read_mode());

            reader.mark_as_read(1);

            // Nothing has come, and what is left is still there
            assert_eq!(reader.get_next().await.unwrap(), "b");
            assert!(!reader.read_mode());
            assert_eq!(requests.load(Ordering::Relaxed), 2);

            // The stream which is over is not asked again
            assert_eq!(reader.get_next().await.unwrap(), "b");
            assert_eq!(reader.as_slice(), b"b");
            assert!(!reader.read_mode());
            assert_eq!(requests.load(Ordering::Relaxed), 2);
        });
    }

    #[test]
    fn a_stream_with_nothing_in_it() {
        rt().block_on(async {
            let mut reader = BufferedReader::new(Chunks::of(&[]));

            assert!(reader.read_mode());
            assert!(reader.get_next().await.unwrap().is_empty());
            assert!(!reader.read_mode());
            assert!(reader.as_slice().is_empty());
        });
    }

    #[test]
    fn a_chunk_is_taken_as_it_is_when_nothing_is_left_to_parse() {
        rt().block_on(async {
            let chunks = [
                Bytes::from(b"Hello".to_vec()),
                Bytes::from(b"World".to_vec()),
            ];
            let ptrs = [chunks[0].as_ptr(), chunks[1].as_ptr()];

            let mut reader = BufferedReader::new(Chunks::new(chunks.into_iter().map(Ok).collect()));

            // The very same buffer - in the reader, and in what it has returned
            let data = reader.get_next().await.unwrap();
            assert_eq!(reader.as_slice().as_ptr(), ptrs[0]);
            assert_eq!(data.as_ptr(), ptrs[0]);
            drop(data);

            // Marking as read moves nothing
            reader.mark_as_read(2);
            assert_eq!(reader.as_slice().as_ptr(), ptrs[0].wrapping_add(2));

            reader.mark_as_read(3);
            reader.get_next().await.unwrap();

            assert_eq!(reader.as_slice(), b"World");
            assert_eq!(reader.as_slice().as_ptr(), ptrs[1]);
        });
    }

    #[test]
    fn a_chunk_is_put_in_place_when_nobody_else_holds_the_buffer() {
        rt().block_on(async {
            // The buffer has the room for what comes next
            let mut first = Vec::with_capacity(64);
            first.extend_from_slice(b"Hello");
            let first_ptr = first.as_ptr();

            let mut reader = BufferedReader::new(Chunks::new(vec![
                Ok(first.into()),
                Ok(Bytes::from_static(b"World")),
            ]));

            // What `get_next()` returns is dropped at once - the reader is the only
            // one who holds the buffer
            reader.get_next().await.unwrap();
            reader.mark_as_read(3);

            reader.get_next().await.unwrap();

            assert_eq!(reader.as_slice(), b"loWorld");
            // The bytes which are not parsed yet are where they were
            assert_eq!(reader.as_slice().as_ptr(), first_ptr.wrapping_add(3));
        });
    }

    #[test]
    fn a_piece_which_is_kept_stays_as_it_is() {
        rt().block_on(async {
            let mut reader = BufferedReader::new(Chunks::of(&["Hello", "World", "!"]));

            let hello = reader.get_next().await.unwrap();
            reader.mark_as_read(3);

            // The buffer is held by `hello` - the bytes which are not parsed yet go
            // to a new one together with the chunk
            let lo_world = reader.get_next().await.unwrap();

            assert_eq!(hello, "Hello");
            assert_eq!(lo_world, "loWorld");
            assert_ne!(lo_world.as_ptr(), hello[3..].as_ptr());

            reader.mark_as_read(2);
            reader.get_next().await.unwrap();

            assert_eq!(reader.as_slice(), b"World!");
            assert_eq!(hello, "Hello");
            assert_eq!(lo_world, "loWorld");
        });
    }

    #[test]
    fn an_error_of_the_stream_loses_nothing() {
        rt().block_on(async {
            let mut reader = BufferedReader::new(Chunks::new(vec![
                Ok(Bytes::from_static(b"ab")),
                Err("no connection".to_string()),
                Ok(Bytes::from_static(b"cd")),
            ]));

            assert_eq!(reader.get_next().await.unwrap(), "ab");
            assert_eq!(reader.get_next().await, Err("no connection".to_string()));

            // What was read before the error is there, and it is not the end
            assert_eq!(reader.as_slice(), b"ab");
            assert!(reader.read_mode());

            assert_eq!(reader.get_next().await.unwrap(), "abcd");
        });
    }

    #[test]
    fn a_chunk_with_nothing_in_it_is_passed_by() {
        rt().block_on(async {
            let src = Chunks::of(&["", "ab", ""]);
            let requests = src.requests.clone();
            let mut reader = BufferedReader::new(src);

            // One call - and there is something to parse
            assert_eq!(reader.get_next().await.unwrap(), "ab");
            assert_eq!(requests.load(Ordering::Relaxed), 2);

            assert_eq!(reader.get_next().await.unwrap(), "ab");
            assert!(!reader.read_mode());
            assert_eq!(requests.load(Ordering::Relaxed), 4);
        });
    }

    #[test]
    #[should_panic(expected = "mark_as_read: 3 bytes are marked as read, and there are 2 of them")]
    fn mark_as_read_of_more_than_there_is_panics() {
        rt().block_on(async {
            let mut reader = BufferedReader::new(Chunks::of(&["ab"]));
            reader.get_next().await.unwrap();

            reader.mark_as_read(3);
        });
    }

    #[test]
    fn it_is_read_from_a_spawned_task() {
        rt().block_on(async {
            let src: Arc<dyn AsyncBytesStream<String> + Send + Sync + 'static> =
                Arc::new(Chunks::of(&["Hello", "World"]));

            let mut reader = BufferedReader::new(src);

            // The reader is `Send`, and so is the future of `get_next()`
            let result = tokio::spawn(async move {
                reader.get_next().await.unwrap();
                reader.mark_as_read(4);
                reader.get_next().await.unwrap()
            })
            .await
            .unwrap();

            assert_eq!(result, "oWorld");
        });
    }

    /// Reads the lines the way a parser does: takes what is complete, and asks for
    /// more when nothing is. `keep` holds on to all that `get_next()` returns.
    async fn read_lines(reader: &mut BufferedReader<String, Chunks>, keep: bool) -> Vec<String> {
        let mut lines = Vec::new();
        let mut kept = Vec::new();

        loop {
            if let Some(end) = reader.as_slice().iter().position(|b| *b == b'\n') {
                lines.push(String::from_utf8(reader.as_slice()[..end].to_vec()).unwrap());
                reader.mark_as_read(end + 1);
                continue;
            }

            if !reader.read_mode() {
                break;
            }

            let data = reader.get_next().await.unwrap();

            if keep {
                kept.push((data.clone(), data.to_vec()));
            }
        }

        // Whatever was read after it, what is kept is what it was
        for (data, as_it_was) in kept {
            assert_eq!(data, as_it_was);
        }

        lines
    }

    #[test]
    fn lines_are_read_however_the_chunks_are_cut() {
        const SRC: &str =
            "{\"id\":1}\n{\"id\":22,\"name\":\"some name\"}\n\n{\"id\":333}\nnot complete";

        rt().block_on(async {
            for chunk_size in 1..=SRC.len() + 1 {
                // `shared` - the chunks are parts of one buffer which is still held,
                // the way they are cut out of what is read off a socket
                for (keep, shared) in [(false, false), (false, true), (true, false), (true, true)] {
                    let read_off_the_socket = Bytes::from(SRC.as_bytes().to_vec());

                    let chunks = (0..SRC.len())
                        .step_by(chunk_size)
                        .map(|from| from..SRC.len().min(from + chunk_size))
                        .map(|range| match shared {
                            true => read_off_the_socket.slice(range),
                            false => Bytes::from(SRC.as_bytes()[range].to_vec()),
                        })
                        .map(Ok)
                        .collect();

                    let mut reader = BufferedReader::new(Chunks::new(chunks));

                    assert_eq!(
                        read_lines(&mut reader, keep).await,
                        [
                            "{\"id\":1}",
                            "{\"id\":22,\"name\":\"some name\"}",
                            "",
                            "{\"id\":333}"
                        ],
                        "chunk_size: {}, keep: {}, shared: {}",
                        chunk_size,
                        keep,
                        shared
                    );

                    // A line which is cut short is left as it is
                    assert_eq!(reader.as_slice(), b"not complete");
                    assert_eq!(read_off_the_socket, SRC);
                }
            }
        });
    }
}
