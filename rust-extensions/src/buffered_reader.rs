use crate::AsyncBytesStream;

/// Reads an [`AsyncBytesStream`] for the one who parses it - a JSON after a JSON, a
/// line after a line, a frame after a frame.
///
/// A stream is cut into chunks where it happens to be, so what is being parsed may
/// begin in one chunk and end in the next one. The reader keeps what is read and not
/// parsed yet in a buffer of its own, as one run of bytes, whatever the chunks were:
/// - `as_slice()` is what there is to parse;
/// - `mark_as_read()` drops what is parsed from the beginning of it;
/// - `get_next()` reads the next chunk in when there is not enough to parse;
/// - `read_mode()` is `false` once the stream is read to its end.
///
/// A chunk is copied into the buffer as soon as it comes, and dropped: the reader
/// holds none of the chunks of the stream, so a [`crate::DoubleBuffer`] has its
/// buffer back at once. The buffer of the reader is reused: what is parsed gives its
/// room back, and it grows the way a `Vec` does.
pub struct BufferedReader<TError, TStream: AsyncBytesStream<TError>> {
    stream: TStream,
    /// What is read. `buffer[parsed..]` is not parsed yet
    buffer: Vec<u8>,
    parsed: usize,
    read_mode: bool,
    // `TError` is what `get_next()` fails with - the reader keeps none of it
    error: std::marker::PhantomData<fn() -> TError>,
}

impl<TError, TStream: AsyncBytesStream<TError>> BufferedReader<TError, TStream> {
    /// Nothing is read until `get_next()` is called.
    pub fn new(stream: TStream) -> Self {
        Self {
            stream,
            buffer: Vec::new(),
            parsed: 0,
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
    /// An error of the stream is returned as it is. Nothing which is read is lost,
    /// and the next call asks the stream again.
    pub async fn get_next(&mut self) -> Result<&[u8], TError> {
        while self.read_mode {
            match self.stream.get_next().await? {
                Some(chunk) if chunk.is_empty() => {}
                Some(chunk) => {
                    append(&mut self.buffer, &mut self.parsed, &chunk);
                    break;
                }
                None => self.read_mode = false,
            }
        }

        Ok(self.as_slice())
    }

    /// What is read and not marked as read yet.
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer[self.parsed..]
    }

    /// `false` - the stream is read to its end: `get_next()` brings nothing in any
    /// more, and what `as_slice()` gives is all that is left.
    pub fn read_mode(&self) -> bool {
        self.read_mode
    }

    /// Drops `size` bytes from the beginning of what is read - they are parsed.
    ///
    /// Panics when there are fewer than `size` bytes.
    pub fn mark_as_read(&mut self, size: usize) {
        assert!(
            size <= self.as_slice().len(),
            "mark_as_read: {} bytes are marked as read, and there are {} of them",
            size,
            self.as_slice().len()
        );

        self.parsed += size;
    }
}

/// Puts the chunk behind the bytes which are not parsed yet. The parsed ones give
/// their room back first: the bytes which are left are moved to the beginning
fn append(buffer: &mut Vec<u8>, parsed: &mut usize, chunk: &[u8]) {
    buffer.drain(..*parsed);
    *parsed = 0;

    buffer.extend_from_slice(chunk);
}

#[cfg(all(test, feature = "with-tokio"))]
mod tests {
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use parking_lot::Mutex;

    use super::BufferedReader;
    use crate::AsyncBytesStream;

    /// Hands its chunks over one by one, and counts how many times it is asked
    struct Chunks {
        chunks: Mutex<VecDeque<Result<Vec<u8>, String>>>,
        requests: AtomicUsize,
    }

    impl Chunks {
        fn new(chunks: Vec<Result<Vec<u8>, String>>) -> Self {
            Self {
                chunks: Mutex::new(chunks.into()),
                requests: AtomicUsize::new(0),
            }
        }

        fn requests(&self) -> usize {
            self.requests.load(Ordering::Relaxed)
        }

        fn of(chunks: &[&str]) -> Self {
            Self::new(
                chunks
                    .iter()
                    .map(|chunk| Ok(chunk.as_bytes().to_vec()))
                    .collect(),
            )
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

            assert_eq!(reader.get_next().await.unwrap(), b"Hello");
            assert_eq!(reader.as_slice(), b"Hello");

            reader.mark_as_read(2);
            assert_eq!(reader.as_slice(), b"llo");

            // What is not parsed yet and the next chunk are one run of bytes
            assert_eq!(reader.get_next().await.unwrap(), b"lloWorld");
            assert_eq!(reader.as_slice(), b"lloWorld");

            reader.mark_as_read(8);
            assert!(reader.as_slice().is_empty());
        });
    }

    #[test]
    fn the_end_of_the_stream_turns_read_mode_off() {
        rt().block_on(async {
            let mut reader = BufferedReader::new(Chunks::of(&["ab"]));

            assert_eq!(reader.get_next().await.unwrap(), b"ab");
            // Nobody knows that it was the last one - until the stream says so
            assert!(reader.read_mode());

            reader.mark_as_read(1);

            // Nothing has come, and what is left is still there
            assert_eq!(reader.get_next().await.unwrap(), b"b");
            assert!(!reader.read_mode());
            assert_eq!(reader.stream.requests(), 2);

            // The stream which is over is not asked again
            assert_eq!(reader.get_next().await.unwrap(), b"b");
            assert_eq!(reader.as_slice(), b"b");
            assert!(!reader.read_mode());
            assert_eq!(reader.stream.requests(), 2);
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
    fn the_room_of_what_is_parsed_is_reused() {
        rt().block_on(async {
            let lines: Vec<String> = (0..1000).map(|no| format!("line{:03}\n", no)).collect();
            let lines: Vec<&str> = lines.iter().map(String::as_str).collect();

            let mut reader = BufferedReader::new(Chunks::of(&lines));

            for line in &lines {
                assert_eq!(reader.get_next().await.unwrap(), line.as_bytes());
                reader.mark_as_read(line.len());
            }

            // A thousand lines went through a buffer of a line or two
            assert!(reader.buffer.capacity() <= 2 * lines[0].len());
        });
    }

    #[test]
    fn an_error_of_the_stream_loses_nothing() {
        rt().block_on(async {
            let mut reader = BufferedReader::new(Chunks::new(vec![
                Ok(b"ab".to_vec()),
                Err("no connection".to_string()),
                Ok(b"cd".to_vec()),
            ]));

            assert_eq!(reader.get_next().await.unwrap(), b"ab");
            assert_eq!(reader.get_next().await, Err("no connection".to_string()));

            // What was read before the error is there, and it is not the end
            assert_eq!(reader.as_slice(), b"ab");
            assert!(reader.read_mode());

            assert_eq!(reader.get_next().await.unwrap(), b"abcd");
        });
    }

    #[test]
    fn a_chunk_with_nothing_in_it_is_passed_by() {
        rt().block_on(async {
            let mut reader = BufferedReader::new(Chunks::of(&["", "ab", ""]));

            // One call - and there is something to parse
            assert_eq!(reader.get_next().await.unwrap(), b"ab");
            assert_eq!(reader.stream.requests(), 2);

            assert_eq!(reader.get_next().await.unwrap(), b"ab");
            assert!(!reader.read_mode());
            assert_eq!(reader.stream.requests(), 4);
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
            let src: Arc<dyn AsyncBytesStream<String, Chunk = Vec<u8>> + Send + Sync> =
                Arc::new(Chunks::of(&["Hello", "World"]));

            let mut reader = BufferedReader::new(src);

            // The reader is `Send`, and so is the future of `get_next()`
            let result = tokio::spawn(async move {
                reader.get_next().await.unwrap();
                reader.mark_as_read(4);
                reader.get_next().await.unwrap().to_vec()
            })
            .await
            .unwrap();

            assert_eq!(result, b"oWorld");
        });
    }

    /// Reads the lines the way a parser does: takes what is complete, and asks for
    /// more when nothing is
    async fn read_lines(reader: &mut BufferedReader<String, Chunks>) -> Vec<String> {
        let mut lines = Vec::new();

        loop {
            if let Some(end) = reader.as_slice().iter().position(|b| *b == b'\n') {
                lines.push(String::from_utf8(reader.as_slice()[..end].to_vec()).unwrap());
                reader.mark_as_read(end + 1);
                continue;
            }

            if !reader.read_mode() {
                break;
            }

            reader.get_next().await.unwrap();
        }

        lines
    }

    #[test]
    fn lines_are_read_however_the_chunks_are_cut() {
        const SRC: &str =
            "{\"id\":1}\n{\"id\":22,\"name\":\"some name\"}\n\n{\"id\":333}\nnot complete";

        rt().block_on(async {
            for chunk_size in 1..=SRC.len() + 1 {
                let chunks = SRC
                    .as_bytes()
                    .chunks(chunk_size)
                    .map(|chunk| Ok(chunk.to_vec()))
                    .collect();

                let mut reader = BufferedReader::new(Chunks::new(chunks));

                assert_eq!(
                    read_lines(&mut reader).await,
                    [
                        "{\"id\":1}",
                        "{\"id\":22,\"name\":\"some name\"}",
                        "",
                        "{\"id\":333}"
                    ],
                    "chunk_size: {}",
                    chunk_size
                );

                // A line which is cut short is left as it is
                assert_eq!(reader.as_slice(), b"not complete");
            }
        });
    }
}
