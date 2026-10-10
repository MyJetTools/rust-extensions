use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::io::AsyncReadExt;

use crate::AsyncBytesStream;

/// A file read as an [`AsyncBytesStream`] - `chunk_size` bytes a chunk: 64 KB, unless
/// [`FileStreamReader::builder()`] sets another size.
///
/// Every chunk is `chunk_size` bytes, and only the last one of the file is shorter.
/// `get_next()` gives `Ok(None)` once the file is read to its end, and `get_size()` is
/// the size of the file when it was opened.
///
/// A chunk is read into a buffer of the reader, and dropping the chunk gives the buffer
/// back: the next chunk is read into it. So a file read a chunk at a time is read with
/// one buffer, allocated once. A chunk which is kept keeps its buffer, and the next
/// chunk is read into a new one.
///
/// A `get_next()` which is given up - a `select!` which took the other branch, a
/// timeout - or which has failed loses nothing: what it has read is kept, and the next
/// `get_next()` goes on from there.
///
/// The file is read through `tokio::fs` - under a tokio runtime.
pub struct FileStreamReader {
    reading: tokio::sync::Mutex<Reading>,
    /// The buffer of a chunk which is processed - the next chunk is read into it
    free_buffer: Arc<Mutex<Option<Vec<u8>>>>,
    chunk_size: usize,
    size: Option<usize>,
}

/// The file, and the chunk which is being read from it - one `get_next()` at a time
struct Reading {
    file: tokio::fs::File,
    /// The buffer the next chunk is read into - empty until it is taken. `buffer[..read]`
    /// is read already: by a `get_next()` which was given up, or which has failed
    buffer: Vec<u8>,
    read: usize,
}

impl FileStreamReader {
    /// The size of a chunk unless the builder sets another one - 64 KB
    pub const DEFAULT_CHUNK_SIZE: usize = 64 * 1024;

    /// Opens the file to be read 64 KB a chunk. Nothing is read until `get_next()` is
    /// called.
    pub async fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        Self::builder().open(path).await
    }

    /// Sets the size of a chunk - `open()` of the builder opens the file then.
    pub fn builder() -> FileStreamReaderBuilder {
        FileStreamReaderBuilder {
            chunk_size: Self::DEFAULT_CHUNK_SIZE,
        }
    }

    /// The buffer to read the next chunk into - the one of a chunk which is processed,
    /// or a new one
    fn take_buffer(&self) -> Vec<u8> {
        let free_buffer = self.free_buffer.lock().take();

        match free_buffer {
            // It may be of the last chunk of the file, which is shorter - it gets its
            // size back
            Some(mut buffer) => {
                buffer.resize(self.chunk_size, 0);
                buffer
            }
            None => vec![0; self.chunk_size],
        }
    }
}

/// The file is a stream: its chunks are `chunk_size` bytes of it one after another, and
/// its size is the size of the file when it was opened.
#[async_trait::async_trait]
impl AsyncBytesStream<std::io::Error> for FileStreamReader {
    type Chunk = FileStreamChunk;

    async fn get_next(&self) -> std::io::Result<Option<FileStreamChunk>> {
        let mut reading = self.reading.lock().await;
        let Reading { file, buffer, read } = &mut *reading;

        if buffer.is_empty() {
            *buffer = self.take_buffer();
        }

        // A read may give fewer bytes than it is asked for - the chunk is read until it
        // is full, or until the file is over
        while *read < buffer.len() {
            match file.read(&mut buffer[*read..]).await? {
                0 => break,
                size => *read += size,
            }
        }

        if *read == 0 {
            return Ok(None);
        }

        let mut chunk = std::mem::take(buffer);
        chunk.truncate(std::mem::take(read));

        Ok(Some(FileStreamChunk {
            buffer: chunk,
            free_buffer: self.free_buffer.clone(),
        }))
    }

    fn get_size(&self) -> Option<usize> {
        self.size
    }
}

/// How a [`FileStreamReader`] reads its file - the size of a chunk. It is `Copy`, and
/// one builder opens as many files as needed.
#[derive(Debug, Clone, Copy)]
pub struct FileStreamReaderBuilder {
    chunk_size: usize,
}

impl FileStreamReaderBuilder {
    /// Every chunk is `chunk_size` bytes, only the last one of the file is shorter.
    ///
    /// Panics when `chunk_size` is 0: nothing could be read into such a buffer, and a
    /// read of nothing is how the end of a file is told.
    pub fn chunk_size(mut self, chunk_size: usize) -> Self {
        assert!(
            chunk_size > 0,
            "FileStreamReaderBuilder::chunk_size: the size of a chunk is 0"
        );

        self.chunk_size = chunk_size;
        self
    }

    /// Opens the file. Nothing is read until `get_next()` is called.
    pub async fn open(self, path: impl AsRef<Path>) -> std::io::Result<FileStreamReader> {
        let file = tokio::fs::File::open(path).await?;
        let metadata = file.metadata().await?;

        // A pipe or a device has no size until it is read to its end
        let size = if metadata.is_file() {
            usize::try_from(metadata.len()).ok()
        } else {
            None
        };

        Ok(FileStreamReader {
            reading: tokio::sync::Mutex::new(Reading {
                file,
                buffer: Vec::new(),
                read: 0,
            }),
            free_buffer: Arc::new(Mutex::new(None)),
            chunk_size: self.chunk_size,
            size,
        })
    }
}

impl Default for FileStreamReaderBuilder {
    fn default() -> Self {
        FileStreamReader::builder()
    }
}

/// What is read - `chunk_size` bytes of the file, fewer at its end, as a `&[u8]`
/// through `Deref`.
///
/// It holds a buffer of the reader. Dropping it says that it is processed: the buffer
/// goes back to the reader, and the next chunk is read into it.
pub struct FileStreamChunk {
    buffer: Vec<u8>,
    free_buffer: Arc<Mutex<Option<Vec<u8>>>>,
}

impl FileStreamChunk {
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
    }
}

impl Deref for FileStreamChunk {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl AsRef<[u8]> for FileStreamChunk {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl Drop for FileStreamChunk {
    fn drop(&mut self) {
        let buffer = std::mem::take(&mut self.buffer);
        let mut free_buffer = self.free_buffer.lock();

        // One is enough for the next chunk - the buffer of a chunk dropped while another
        // one is kept there is let go
        if free_buffer.is_none() {
            *free_buffer = Some(buffer);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};
    use std::time::{Duration, Instant};

    use super::{FileStreamChunk, FileStreamReader};
    use crate::{AsyncBytesStream, BufferedReader};

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
    }

    /// A file in the temp dir, removed on drop.
    struct TempFile(std::path::PathBuf);

    impl TempFile {
        fn path_of(name: &str) -> std::path::PathBuf {
            std::env::temp_dir().join(format!(
                "rust-extensions-file-stream-reader-{}-{}",
                std::process::id(),
                name
            ))
        }

        fn new(name: &str, content: &[u8]) -> Self {
            let path = Self::path_of(name);
            std::fs::write(&path, content).unwrap();
            Self(path)
        }

        /// A named pipe - what is written into it is read, and it has no size
        #[cfg(unix)]
        fn fifo(name: &str) -> Self {
            let path = Self::path_of(name);
            let _ = std::fs::remove_file(&path);

            let status = std::process::Command::new("mkfifo")
                .arg(&path)
                .status()
                .unwrap();
            assert!(status.success());

            Self(path)
        }

        fn path(&self) -> &str {
            self.0.to_str().unwrap()
        }

        fn append(&self, content: &[u8]) {
            use std::io::Write;

            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&self.0)
                .unwrap();
            file.write_all(content).unwrap();
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    /// The next chunk, processed at once: a copy of it
    async fn next(reader: &FileStreamReader) -> Option<Vec<u8>> {
        reader.get_next().await.unwrap().map(|chunk| chunk.to_vec())
    }

    /// Bytes which tell where in the file they are
    fn content(size: usize) -> Vec<u8> {
        (0..size).map(|pos| (pos % 251) as u8).collect()
    }

    /// The buffer the reader keeps for the next chunk. Its address alone tells nothing:
    /// a buffer which is let go and a new one may well get the same.
    fn kept_buffer(reader: &FileStreamReader) -> Option<*const u8> {
        reader
            .free_buffer
            .lock()
            .as_ref()
            .map(|buffer| buffer.as_ptr())
    }

    #[test]
    fn reads_the_file_64_kb_a_chunk() {
        assert_eq!(FileStreamReader::DEFAULT_CHUNK_SIZE, 64 * 1024);

        let content = content(2 * 64 * 1024 + 100);
        let file = TempFile::new("default", &content);

        rt().block_on(async {
            let reader = FileStreamReader::open(file.path()).await.unwrap();

            assert_eq!(next(&reader).await.as_deref(), Some(&content[..65536]));
            assert_eq!(
                next(&reader).await.as_deref(),
                Some(&content[65536..131072])
            );
            // Only the last one is shorter
            assert_eq!(next(&reader).await.as_deref(), Some(&content[131072..]));

            assert_eq!(next(&reader).await, None);
            assert_eq!(next(&reader).await, None);
        });
    }

    #[test]
    fn a_chunk_bigger_than_a_read_is_read_to_its_size() {
        // tokio reads a file 2 MB at most a read
        const CHUNK_SIZE: usize = 3 * 1024 * 1024;

        let content = content(CHUNK_SIZE + 1024 * 1024);
        let file = TempFile::new("big-chunk", &content);

        rt().block_on(async {
            let reader = FileStreamReader::builder()
                .chunk_size(CHUNK_SIZE)
                .open(file.path())
                .await
                .unwrap();

            assert_eq!(next(&reader).await.as_deref(), Some(&content[..CHUNK_SIZE]));
            assert_eq!(next(&reader).await.as_deref(), Some(&content[CHUNK_SIZE..]));
            assert_eq!(next(&reader).await, None);
        });
    }

    #[test]
    fn the_builder_sets_the_size_of_a_chunk() {
        let file = TempFile::new("builder", b"0123456789");

        rt().block_on(async {
            let reader = FileStreamReader::builder()
                .chunk_size(4)
                .open(file.path())
                .await
                .unwrap();

            assert_eq!(next(&reader).await, Some(b"0123".to_vec()));
            assert_eq!(next(&reader).await, Some(b"4567".to_vec()));
            assert_eq!(next(&reader).await, Some(b"89".to_vec()));
            assert_eq!(next(&reader).await, None);
        });
    }

    #[test]
    fn one_builder_opens_many_files() {
        let first = TempFile::new("first", b"0123");
        let second = TempFile::new("second", b"abcd");

        rt().block_on(async {
            let builder = FileStreamReader::builder().chunk_size(2);

            let first = builder.open(first.path()).await.unwrap();
            let second = builder.open(second.path()).await.unwrap();

            assert_eq!(next(&first).await, Some(b"01".to_vec()));
            assert_eq!(next(&second).await, Some(b"ab".to_vec()));
        });
    }

    #[test]
    fn a_file_of_whole_chunks_ends_with_none() {
        let file = TempFile::new("whole-chunks", b"01234567");

        rt().block_on(async {
            let reader = FileStreamReader::builder()
                .chunk_size(4)
                .open(file.path())
                .await
                .unwrap();

            assert_eq!(next(&reader).await, Some(b"0123".to_vec()));
            assert_eq!(next(&reader).await, Some(b"4567".to_vec()));
            // Not an empty chunk
            assert_eq!(next(&reader).await, None);
        });
    }

    #[test]
    fn an_empty_file_is_a_stream_with_nothing_in_it() {
        let file = TempFile::new("empty", b"");

        rt().block_on(async {
            let reader = FileStreamReader::open(file.path()).await.unwrap();

            assert_eq!(reader.get_size(), Some(0));
            assert_eq!(next(&reader).await, None);

            let reader = FileStreamReader::open(file.path()).await.unwrap();
            assert_eq!(reader.into_vec().await.unwrap(), b"");
        });
    }

    #[test]
    fn into_vec_reads_the_file_into_a_vec_of_its_size() {
        let content = content(100_000);
        let file = TempFile::new("into-vec", &content);

        rt().block_on(async {
            let reader = FileStreamReader::builder()
                .chunk_size(4096)
                .open(file.path())
                .await
                .unwrap();
            assert_eq!(reader.get_size(), Some(100_000));

            let result = reader.into_vec().await.unwrap();

            assert_eq!(result, content);
            assert!(result.capacity() >= 100_000);
        });
    }

    #[test]
    fn the_buffer_of_a_dropped_chunk_is_read_into_again() {
        let file = TempFile::new("reused", b"0123456789");

        rt().block_on(async {
            let reader = FileStreamReader::builder()
                .chunk_size(4)
                .open(file.path())
                .await
                .unwrap();

            let first = reader.get_next().await.unwrap().unwrap();
            let buffer_ptr = first.as_ptr();
            assert_eq!(kept_buffer(&reader), None);

            // Given back, and kept for the next chunk
            drop(first);
            assert_eq!(kept_buffer(&reader), Some(buffer_ptr));

            let second = reader.get_next().await.unwrap().unwrap();
            assert_eq!(&*second, b"4567");
            // The very same buffer
            assert_eq!(second.as_ptr(), buffer_ptr);
            assert_eq!(kept_buffer(&reader), None);
        });
    }

    #[test]
    fn a_kept_chunk_keeps_its_buffer() {
        let file = TempFile::new("kept", b"0123456789");

        rt().block_on(async {
            let reader = FileStreamReader::builder()
                .chunk_size(4)
                .open(file.path())
                .await
                .unwrap();

            let first = reader.get_next().await.unwrap().unwrap();
            let second = reader.get_next().await.unwrap().unwrap();

            // The second one is read into a buffer of its own
            assert_ne!(first.as_ptr(), second.as_ptr());
            assert_eq!(&*first, b"0123");
            assert_eq!(&*second, b"4567");

            let first_ptr = first.as_ptr();
            drop(first);
            assert_eq!(kept_buffer(&reader), Some(first_ptr));

            // One is kept - the buffer of the one dropped first
            drop(second);
            assert_eq!(kept_buffer(&reader), Some(first_ptr));

            let third = reader.get_next().await.unwrap().unwrap();
            assert_eq!(&*third, b"89");
            assert_eq!(third.as_ptr(), first_ptr);
        });
    }

    #[test]
    fn the_buffer_of_a_short_chunk_gets_its_size_back() {
        let file = TempFile::new("short", b"012345");

        rt().block_on(async {
            let reader = FileStreamReader::builder()
                .chunk_size(4)
                .open(file.path())
                .await
                .unwrap();

            assert_eq!(next(&reader).await, Some(b"0123".to_vec()));
            // The end of the file - the buffer is given back two bytes long
            assert_eq!(next(&reader).await, Some(b"45".to_vec()));
            assert_eq!(next(&reader).await, None);

            // The file has grown - and the chunk is four bytes again
            file.append(b"6789ab");

            assert_eq!(next(&reader).await, Some(b"6789".to_vec()));
            assert_eq!(next(&reader).await, Some(b"ab".to_vec()));
            assert_eq!(next(&reader).await, None);
        });
    }

    #[test]
    fn a_chunk_outlives_the_reader() {
        let file = TempFile::new("outlives", b"0123");

        rt().block_on(async {
            let reader = FileStreamReader::open(file.path()).await.unwrap();
            let chunk = reader.get_next().await.unwrap().unwrap();

            drop(reader);

            assert_eq!(&*chunk, b"0123");
        });
    }

    #[test]
    fn calls_from_many_tasks_read_one_chunk_after_another() {
        const CHUNKS: usize = 64;
        const CHUNK_SIZE: usize = 1024;

        // Every chunk is filled with its number
        let content: Vec<u8> = (0..CHUNKS).flat_map(|no| [no as u8; CHUNK_SIZE]).collect();
        let file = TempFile::new("many-tasks", &content);

        rt().block_on(async {
            let reader = FileStreamReader::builder()
                .chunk_size(CHUNK_SIZE)
                .open(file.path())
                .await
                .unwrap();
            let reader = Arc::new(reader);

            let tasks: Vec<_> = (0..8)
                .map(|_| {
                    let reader = reader.clone();

                    tokio::spawn(async move {
                        let mut read = Vec::new();

                        while let Some(chunk) = reader.get_next().await.unwrap() {
                            // A chunk is one piece of the file, not parts of two
                            assert_eq!(chunk.len(), CHUNK_SIZE);
                            assert!(chunk.iter().all(|b| *b == chunk[0]));
                            read.push(chunk[0] as usize);

                            // The chunk is kept while the others read
                            tokio::task::yield_now().await;
                        }

                        read
                    })
                })
                .collect();

            let mut read = Vec::new();
            for task in tasks {
                read.extend(task.await.unwrap());
            }

            // Every chunk once
            read.sort_unstable();
            assert_eq!(read, (0..CHUNKS).collect::<Vec<_>>());
        });
    }

    /// Counts how many times the one who waits is woken up
    #[derive(Default)]
    struct WakeUps(AtomicUsize);

    impl WakeUps {
        fn count(&self) -> usize {
            self.0.load(Ordering::SeqCst)
        }

        /// Waits until the one who waits is woken up `count` times
        fn wait_for(&self, count: usize) {
            let started = Instant::now();

            while self.count() < count {
                assert!(
                    started.elapsed() < Duration::from_secs(10),
                    "it is never woken up"
                );
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }

    impl Wake for WakeUps {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn poll<T>(fut: Pin<&mut impl Future<Output = T>>, wake_ups: &Arc<WakeUps>) -> Poll<T> {
        let waker = Waker::from(wake_ups.clone());
        fut.poll(&mut Context::from_waker(&waker))
    }

    /// A pipe gives what is written into it, and waits for more - so the chunk is read
    /// in parts, and the `get_next()` is given up while it waits for the rest
    #[cfg(unix)]
    #[test]
    fn a_get_next_which_is_given_up_loses_nothing() {
        let fifo = TempFile::fifo("given-up");

        // The one who writes into the pipe: what it is told to. The end of the pipe is
        // once the sender is dropped - by the test, or by a panic in it
        let (tell, told) = std::sync::mpsc::channel::<&'static [u8]>();
        let path = fifo.path().to_string();
        let writing = std::thread::spawn(move || {
            use std::io::Write;

            let mut pipe = std::fs::OpenOptions::new().write(true).open(path).unwrap();

            for data in told {
                pipe.write_all(data).unwrap();
            }
        });

        let rt = rt();
        let reader = rt
            .block_on(FileStreamReader::builder().chunk_size(10).open(fifo.path()))
            .unwrap();

        // A pipe has no size
        assert_eq!(reader.get_size(), None);

        // The reads go to the blocking pool of the runtime
        let _runtime = rt.enter();

        let wake_ups = Arc::new(WakeUps::default());
        let mut next = Box::pin(reader.get_next());

        // It waits for something to be written...
        assert!(poll(next.as_mut(), &wake_ups).is_pending());
        tell.send(b"0123").unwrap();

        // ...a part of the chunk is read, and it waits for the rest
        wake_ups.wait_for(1);
        assert!(poll(next.as_mut(), &wake_ups).is_pending());

        // A `select!` which took the other branch
        drop(next);

        tell.send(b"456789").unwrap();
        drop(tell);

        let chunk = rt.block_on(reader.get_next()).unwrap().unwrap();
        assert_eq!(&*chunk, b"0123456789");
        drop(chunk);

        assert!(rt.block_on(reader.get_next()).unwrap().is_none());

        writing.join().unwrap();
    }

    #[test]
    fn a_file_which_is_not_there() {
        rt().block_on(async {
            let err = FileStreamReader::open(TempFile::path_of("not-there"))
                .await
                .err()
                .unwrap();

            assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
        });
    }

    #[test]
    #[should_panic(expected = "FileStreamReaderBuilder::chunk_size: the size of a chunk is 0")]
    fn a_chunk_of_no_size_panics() {
        let _ = FileStreamReader::builder().chunk_size(0);
    }

    #[test]
    fn it_is_read_from_a_spawned_task() {
        let file = TempFile::new("spawned", b"Hello World");

        rt().block_on(async {
            let src: Arc<
                dyn AsyncBytesStream<std::io::Error, Chunk = FileStreamChunk> + Send + Sync,
            > = Arc::new(FileStreamReader::open(file.path()).await.unwrap());

            // The future of `get_next()` is `Send`, and so is a chunk
            let result = tokio::spawn(async move {
                let chunk = src.get_next().await.unwrap().unwrap();
                tokio::spawn(async move { chunk.to_vec() }).await.unwrap()
            })
            .await
            .unwrap();

            assert_eq!(result, b"Hello World");
        });
    }

    /// Reads the lines the way a parser does: takes what is complete, and asks for
    /// more when nothing is
    async fn read_lines(
        reader: &mut BufferedReader<std::io::Error, FileStreamReader>,
    ) -> Vec<String> {
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
    fn it_is_parsed_through_a_buffered_reader_whatever_the_size_of_a_chunk() {
        const SRC: &str =
            "{\"id\":1}\n{\"id\":22,\"name\":\"some name\"}\n\n{\"id\":333}\nnot complete";

        let file = TempFile::new("parsed", SRC.as_bytes());

        rt().block_on(async {
            for chunk_size in 1..=SRC.len() + 1 {
                let reader = FileStreamReader::builder()
                    .chunk_size(chunk_size)
                    .open(file.path())
                    .await
                    .unwrap();

                let mut reader = BufferedReader::new(reader);

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
