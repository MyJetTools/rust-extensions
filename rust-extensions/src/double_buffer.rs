use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use bytes::Bytes;
use parking_lot::Mutex;

const BUFFERS: usize = 2;

/// Two buffers between the one who reads a stream and the one who parses it - while
/// a chunk is being parsed, the next one is read into the other buffer.
///
/// `new()` gives the two ends:
/// - [`DoubleBufferWriter`] is for the one who reads: `get_buffer_to_read()` gives a
///   buffer to read into, and `send()` of that buffer hands over what is read;
/// - [`DoubleBufferReader`] is for the one who parses: `get_next()` gives what is
///   read, and once that is dropped its buffer is free to be read into again.
///
/// Both wait. `get_buffer_to_read()` waits until one of the two buffers is free, so
/// the one who reads is never more than two buffers ahead of the one who parses.
/// `get_next()` waits until something is read.
///
/// No bytes are copied, and there are never more than the two buffers.
pub struct DoubleBuffer;

impl DoubleBuffer {
    /// The two ends. Each of the two buffers is `buffer_size` bytes, and it is
    /// allocated when it is taken for the first time.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(buffer_size: usize) -> (DoubleBufferWriter, DoubleBufferReader) {
        let inner = Arc::new(DoubleBufferInner {
            buffer_size,
            state: Mutex::new(State {
                free: Vec::with_capacity(BUFFERS),
                not_created: BUFFERS,
                read: VecDeque::with_capacity(BUFFERS),
                writer_is_dropped: false,
                reader_is_dropped: false,
                waiting_to_read: Vec::new(),
                waiting_to_parse: Vec::new(),
            }),
        });

        (
            DoubleBufferWriter {
                inner: inner.clone(),
            },
            DoubleBufferReader { inner },
        )
    }
}

struct DoubleBufferInner {
    buffer_size: usize,
    state: Mutex<State>,
}

struct State {
    /// The buffers nobody reads into and nobody parses
    free: Vec<Vec<u8>>,
    /// How many of the two buffers are not allocated yet
    not_created: usize,
    /// What is read and not taken to be parsed yet - a buffer and how many bytes of
    /// it are read, in the order they were sent
    read: VecDeque<(Vec<u8>, usize)>,
    writer_is_dropped: bool,
    reader_is_dropped: bool,
    /// Those who wait for a buffer to read into
    waiting_to_read: Vec<Waker>,
    /// Those who wait for something to parse
    waiting_to_parse: Vec<Waker>,
}

impl DoubleBufferInner {
    /// The buffer is free - the one who waits for a buffer to read into is woken up
    fn free(&self, buffer: Vec<u8>) {
        let waiting = {
            let mut state = self.state.lock();
            state.free.push(buffer);
            std::mem::take(&mut state.waiting_to_read)
        };

        wake(waiting);
    }
}

/// A future which is polled again and again while it waits is kept once
fn wait(waiting: &mut Vec<Waker>, cx: &Context<'_>) {
    if !waiting.iter().any(|waker| waker.will_wake(cx.waker())) {
        waiting.push(cx.waker().clone());
    }
}

/// It is called with the lock released: waking up may drop a task, and a task may
/// hold a buffer - which comes back through the same lock.
///
/// All of them are woken up, since a wait which was given up is among them as well,
/// and it would take the only wake-up away.
fn wake(waiting: Vec<Waker>) {
    for waker in waiting {
        waker.wake();
    }
}

/// The end of a [`DoubleBuffer`] for the one who reads the stream.
///
/// Dropping it is the end of what is read: the one who parses gets what was sent
/// before, and `None` after it.
pub struct DoubleBufferWriter {
    inner: Arc<DoubleBufferInner>,
}

impl DoubleBufferWriter {
    /// A buffer to read the stream into. It waits until one of the two is free -
    /// that is, until the one who parses has dropped what it was given.
    ///
    /// `None` - the reader is dropped, and nobody is going to parse what is read.
    pub async fn get_buffer_to_read(&self) -> Option<BufferToRead<'_>> {
        std::future::poll_fn(|cx| self.poll_buffer_to_read(cx)).await
    }

    fn poll_buffer_to_read(&self, cx: &mut Context<'_>) -> Poll<Option<BufferToRead<'_>>> {
        let buffer = {
            let mut state = self.inner.state.lock();

            if state.reader_is_dropped {
                return Poll::Ready(None);
            }

            match state.free.pop() {
                Some(buffer) => Some(buffer),
                None if state.not_created > 0 => {
                    state.not_created -= 1;
                    None
                }
                None => {
                    wait(&mut state.waiting_to_read, cx);
                    return Poll::Pending;
                }
            }
        };

        // It is allocated with the lock released
        let buffer = buffer.unwrap_or_else(|| vec![0; self.inner.buffer_size]);

        Poll::Ready(Some(BufferToRead {
            buffer: Some(buffer),
            writer: self,
        }))
    }
}

impl Drop for DoubleBufferWriter {
    fn drop(&mut self) {
        let waiting = {
            let mut state = self.inner.state.lock();
            state.writer_is_dropped = true;
            std::mem::take(&mut state.waiting_to_parse)
        };

        wake(waiting);
    }
}

/// A buffer to read the stream into - all the `buffer_size` bytes of it, as a
/// `&mut [u8]`. What was read into it before is still there.
///
/// `send()` hands over what is read. A buffer which is dropped with no `send()` - a
/// read which was given up, say - is free to be taken again.
pub struct BufferToRead<'s> {
    /// `None` - it is sent
    buffer: Option<Vec<u8>>,
    writer: &'s DoubleBufferWriter,
}

impl BufferToRead<'_> {
    /// `size` bytes are read into the beginning of the buffer - they go to the one
    /// who parses, and they are what the next `get_next()` of the reader gives.
    ///
    /// With `size` of `0` there is nothing to parse, and with the reader dropped
    /// there is nobody to parse it: the buffer is just free again.
    ///
    /// Panics when the buffer is smaller than `size` bytes.
    pub fn send(mut self, size: usize) {
        assert!(
            size <= self.len(),
            "send: {} bytes are sent, and the buffer has {} of them",
            size,
            self.len()
        );

        if size == 0 {
            return;
        }

        let waiting = {
            let mut state = self.writer.inner.state.lock();

            if state.reader_is_dropped {
                return;
            }

            if let Some(buffer) = self.buffer.take() {
                state.read.push_back((buffer, size));
            }

            std::mem::take(&mut state.waiting_to_parse)
        };

        wake(waiting);
    }
}

impl Deref for BufferToRead<'_> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.buffer.as_deref().unwrap_or_default()
    }
}

impl DerefMut for BufferToRead<'_> {
    fn deref_mut(&mut self) -> &mut [u8] {
        self.buffer.as_deref_mut().unwrap_or_default()
    }
}

impl Drop for BufferToRead<'_> {
    fn drop(&mut self) {
        // It is not sent - there is nothing to parse in it
        if let Some(buffer) = self.buffer.take() {
            self.writer.inner.free(buffer);
        }
    }
}

/// The end of a [`DoubleBuffer`] for the one who parses the stream.
///
/// Dropping it stops the one who reads: `get_buffer_to_read()` gives `None` from
/// then on.
pub struct DoubleBufferReader {
    inner: Arc<DoubleBufferInner>,
}

impl DoubleBufferReader {
    /// What is read - the chunks come in the order they were sent. It waits until
    /// there is one.
    ///
    /// `None` - the writer is dropped, and all it had sent is given already.
    ///
    /// A chunk holds its buffer, and there are two of them. While both chunks are
    /// held, nothing is read - so `get_next()` called at that moment waits forever.
    pub async fn get_next(&self) -> Option<DoubleBufferChunk> {
        std::future::poll_fn(|cx| self.poll_next(cx)).await
    }

    fn poll_next(&self, cx: &mut Context<'_>) -> Poll<Option<DoubleBufferChunk>> {
        let mut state = self.inner.state.lock();

        if let Some((buffer, size)) = state.read.pop_front() {
            return Poll::Ready(Some(DoubleBufferChunk {
                buffer,
                size,
                inner: self.inner.clone(),
            }));
        }

        if state.writer_is_dropped {
            return Poll::Ready(None);
        }

        wait(&mut state.waiting_to_parse, cx);
        Poll::Pending
    }
}

impl Drop for DoubleBufferReader {
    fn drop(&mut self) {
        let (not_parsed, waiting) = {
            let mut state = self.inner.state.lock();
            state.reader_is_dropped = true;

            (
                std::mem::take(&mut state.read),
                std::mem::take(&mut state.waiting_to_read),
            )
        };

        // Nobody is going to parse it
        drop(not_parsed);
        wake(waiting);
    }
}

/// What is read - the bytes to parse, as a `&[u8]`.
///
/// It holds one of the two buffers. Once it is dropped, the buffer is free to be
/// read into again - that is how the one who parses says that it is done.
pub struct DoubleBufferChunk {
    buffer: Vec<u8>,
    size: usize,
    inner: Arc<DoubleBufferInner>,
}

impl DoubleBufferChunk {
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer[..self.size]
    }

    /// The same bytes as a `Bytes` - what a chunk of an `AsyncBytesStream` is. It is
    /// not a copy: the buffer is held until the last piece of that `Bytes` - a clone
    /// of it, a slice of it - is dropped.
    pub fn into_bytes(self) -> Bytes {
        Bytes::from_owner(self)
    }
}

impl Deref for DoubleBufferChunk {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl AsRef<[u8]> for DoubleBufferChunk {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl Drop for DoubleBufferChunk {
    fn drop(&mut self) {
        self.inner.free(std::mem::take(&mut self.buffer));
    }
}

#[cfg(all(test, feature = "with-tokio"))]
mod tests {
    use std::collections::HashSet;
    use std::future::Future;
    use std::pin::{pin, Pin};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};
    use std::time::Duration;

    use bytes::Bytes;

    use super::{DoubleBuffer, DoubleBufferChunk, DoubleBufferReader, DoubleBufferWriter};
    use crate::{AsyncBytesStream, BufferedReader};

    /// Counts how many times the one who waits is woken up
    #[derive(Default)]
    struct WakeUps(AtomicUsize);

    impl WakeUps {
        fn new() -> Arc<Self> {
            Arc::new(Self::default())
        }

        fn count(&self) -> usize {
            self.0.load(Ordering::Relaxed)
        }
    }

    impl Wake for WakeUps {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn poll<T>(fut: Pin<&mut impl Future<Output = T>>, wake_ups: &Arc<WakeUps>) -> Poll<T> {
        let waker = Waker::from(wake_ups.clone());
        fut.poll(&mut Context::from_waker(&waker))
    }

    /// What the future gives with no waiting
    fn now<T>(fut: impl Future<Output = T>) -> T {
        match poll(pin!(fut), &WakeUps::new()) {
            Poll::Ready(result) => result,
            Poll::Pending => panic!("it waits"),
        }
    }

    /// Reads `src` into a buffer and sends it
    fn send(writer: &DoubleBufferWriter, src: &[u8]) {
        let mut buffer = now(writer.get_buffer_to_read()).unwrap();
        buffer[..src.len()].copy_from_slice(src);
        buffer.send(src.len());
    }

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
    }

    #[test]
    fn what_is_read_comes_to_the_one_who_parses() {
        let (writer, reader) = DoubleBuffer::new(8);

        let mut buffer = now(writer.get_buffer_to_read()).unwrap();
        assert_eq!(buffer.len(), 8);
        let buffer_ptr = buffer.as_ptr();

        buffer[..5].copy_from_slice(b"Hello");
        buffer.send(5);

        let chunk = now(reader.get_next()).unwrap();
        assert_eq!(chunk.as_slice(), b"Hello");
        // The very same buffer - nothing is copied
        assert_eq!(chunk.as_ptr(), buffer_ptr);
    }

    #[test]
    fn chunks_come_in_the_order_they_are_sent() {
        let (writer, reader) = DoubleBuffer::new(4);

        send(&writer, b"ab");
        send(&writer, b"cd");

        assert_eq!(now(reader.get_next()).unwrap().as_slice(), b"ab");
        assert_eq!(now(reader.get_next()).unwrap().as_slice(), b"cd");
    }

    #[test]
    fn the_third_buffer_waits_until_one_of_the_two_is_free() {
        let (writer, reader) = DoubleBuffer::new(4);

        send(&writer, b"ab");
        send(&writer, b"cd");

        let wake_ups = WakeUps::new();
        let mut third = pin!(writer.get_buffer_to_read());
        assert!(poll(third.as_mut(), &wake_ups).is_pending());

        // It is taken to be parsed - and it is not parsed yet
        let chunk = now(reader.get_next()).unwrap();
        let chunk_ptr = chunk.as_ptr();
        assert!(poll(third.as_mut(), &wake_ups).is_pending());
        assert_eq!(wake_ups.count(), 0);

        drop(chunk);
        // Polled twice while it waited - and woken up once
        assert_eq!(wake_ups.count(), 1);

        let Poll::Ready(Some(buffer)) = poll(third.as_mut(), &wake_ups) else {
            panic!("it waits");
        };
        assert_eq!(buffer.as_ptr(), chunk_ptr);
    }

    #[test]
    fn get_next_waits_until_something_is_read() {
        let (writer, reader) = DoubleBuffer::new(4);

        let wake_ups = WakeUps::new();
        let mut next = pin!(reader.get_next());
        assert!(poll(next.as_mut(), &wake_ups).is_pending());

        // A buffer is taken - and nothing is read into it yet
        let mut buffer = now(writer.get_buffer_to_read()).unwrap();
        assert!(poll(next.as_mut(), &wake_ups).is_pending());
        assert_eq!(wake_ups.count(), 0);

        buffer[0] = 7;
        buffer.send(1);
        assert_eq!(wake_ups.count(), 1);

        let Poll::Ready(Some(chunk)) = poll(next.as_mut(), &wake_ups) else {
            panic!("it waits");
        };
        assert_eq!(chunk.as_slice(), [7]);
    }

    #[test]
    fn a_buffer_which_is_not_sent_is_free_again() {
        let (writer, reader) = DoubleBuffer::new(4);

        let first = now(writer.get_buffer_to_read()).unwrap();
        let second = now(writer.get_buffer_to_read()).unwrap();
        let second_ptr = second.as_ptr();

        // A read which was given up
        drop(second);

        let again = now(writer.get_buffer_to_read()).unwrap();
        assert_eq!(again.as_ptr(), second_ptr);
        assert_ne!(again.as_ptr(), first.as_ptr());

        // Nothing has come to be parsed
        assert!(poll(pin!(reader.get_next()), &WakeUps::new()).is_pending());
    }

    #[test]
    fn nothing_read_is_nothing_to_parse() {
        let (writer, reader) = DoubleBuffer::new(4);

        now(writer.get_buffer_to_read()).unwrap().send(0);
        now(writer.get_buffer_to_read()).unwrap().send(0);

        // Both buffers are free again
        let _first = now(writer.get_buffer_to_read()).unwrap();
        let _second = now(writer.get_buffer_to_read()).unwrap();

        assert!(poll(pin!(reader.get_next()), &WakeUps::new()).is_pending());
    }

    #[test]
    fn a_dropped_writer_is_the_end_of_what_is_read() {
        let (writer, reader) = DoubleBuffer::new(4);

        send(&writer, b"ab");
        drop(writer);

        // What was sent before is still there to be parsed
        assert_eq!(now(reader.get_next()).unwrap().as_slice(), b"ab");
        assert!(now(reader.get_next()).is_none());
        assert!(now(reader.get_next()).is_none());
    }

    #[test]
    fn a_dropped_writer_wakes_up_the_one_who_waits_to_parse() {
        let (writer, reader) = DoubleBuffer::new(4);

        let wake_ups = WakeUps::new();
        let mut next = pin!(reader.get_next());
        assert!(poll(next.as_mut(), &wake_ups).is_pending());

        drop(writer);
        assert_eq!(wake_ups.count(), 1);
        assert!(matches!(poll(next.as_mut(), &wake_ups), Poll::Ready(None)));
    }

    #[test]
    fn a_dropped_reader_stops_the_one_who_reads() {
        let (writer, reader) = DoubleBuffer::new(4);

        send(&writer, b"ab");
        let taken = now(writer.get_buffer_to_read()).unwrap();

        let wake_ups = WakeUps::new();
        let mut third = pin!(writer.get_buffer_to_read());
        assert!(poll(third.as_mut(), &wake_ups).is_pending());

        drop(reader);
        assert_eq!(wake_ups.count(), 1);
        assert!(matches!(poll(third.as_mut(), &wake_ups), Poll::Ready(None)));

        // What was taken before goes nowhere
        taken.send(1);
        assert!(now(writer.get_buffer_to_read()).is_none());
    }

    #[test]
    fn a_chunk_outlives_both_ends() {
        let (writer, reader) = DoubleBuffer::new(4);

        send(&writer, b"ab");
        let chunk = now(reader.get_next()).unwrap();

        drop(writer);
        drop(reader);

        assert_eq!(chunk.as_slice(), b"ab");
    }

    #[test]
    fn bytes_hold_the_buffer_until_the_last_piece_of_them_is_dropped() {
        let (writer, reader) = DoubleBuffer::new(8);

        send(&writer, b"Hello");
        send(&writer, b"World");

        let chunk = now(reader.get_next()).unwrap();
        let chunk_ptr = chunk.as_ptr();

        let hello = chunk.into_bytes();
        assert_eq!(hello, "Hello");
        // Not a copy
        assert_eq!(hello.as_ptr(), chunk_ptr);

        let ll = hello.slice(2..4);

        let wake_ups = WakeUps::new();
        let mut third = pin!(writer.get_buffer_to_read());
        assert!(poll(third.as_mut(), &wake_ups).is_pending());

        drop(hello);
        // A slice of it is still held
        assert!(poll(third.as_mut(), &wake_ups).is_pending());
        assert_eq!(wake_ups.count(), 0);
        assert_eq!(ll, "ll");

        drop(ll);
        assert_eq!(wake_ups.count(), 1);

        let Poll::Ready(Some(buffer)) = poll(third.as_mut(), &wake_ups) else {
            panic!("it waits");
        };
        assert_eq!(buffer.as_ptr(), chunk_ptr);
    }

    #[test]
    fn a_wait_which_is_given_up_does_not_take_the_wake_up_away() {
        let (writer, reader) = DoubleBuffer::new(4);

        send(&writer, b"ab");
        send(&writer, b"cd");

        let given_up = WakeUps::new();
        {
            let mut fut = pin!(writer.get_buffer_to_read());
            assert!(poll(fut.as_mut(), &given_up).is_pending());
        }

        let wake_ups = WakeUps::new();
        let mut fut = pin!(writer.get_buffer_to_read());
        assert!(poll(fut.as_mut(), &wake_ups).is_pending());

        drop(now(reader.get_next()));

        assert_eq!(wake_ups.count(), 1);
        assert!(matches!(
            poll(fut.as_mut(), &wake_ups),
            Poll::Ready(Some(_))
        ));
    }

    #[test]
    #[should_panic(expected = "send: 5 bytes are sent, and the buffer has 4 of them")]
    fn send_of_more_than_the_buffer_has_panics() {
        let (writer, _reader) = DoubleBuffer::new(4);

        now(writer.get_buffer_to_read()).unwrap().send(5);
    }

    #[test]
    fn it_is_read_in_one_thread_and_parsed_in_another() {
        const CHUNKS: u32 = 50_000;
        const BUFFER_SIZE: usize = 64;

        let (writer, reader) = DoubleBuffer::new(BUFFER_SIZE);

        let reading = std::thread::spawn(move || {
            rt().block_on(async move {
                for no in 0..CHUNKS {
                    let mut buffer = writer.get_buffer_to_read().await.unwrap();

                    // The number of the chunk, and the size which is told by it
                    let size = 4 + no as usize % (BUFFER_SIZE - 3);
                    buffer[..size].fill(no as u8);
                    buffer[..4].copy_from_slice(&no.to_le_bytes());

                    buffer.send(size);
                }
            })
        });

        let parsing = async {
            let mut buffers = HashSet::new();
            let mut no = 0u32;

            while let Some(chunk) = reader.get_next().await {
                buffers.insert(chunk.as_ptr() as usize);

                // The one who parses is slow now and then - so both get to wait
                if no % 5 == 0 {
                    tokio::task::yield_now().await;
                }

                // Whatever was read meanwhile, the chunk is what it was
                assert_eq!(chunk.len(), 4 + no as usize % (BUFFER_SIZE - 3));
                assert_eq!(chunk[..4], no.to_le_bytes());
                assert!(chunk[4..].iter().all(|b| *b == no as u8));

                no += 1;
            }

            assert_eq!(no, CHUNKS);
            buffers
        };

        let buffers = rt()
            .block_on(async { tokio::time::timeout(Duration::from_secs(60), parsing).await })
            .expect("somebody waits and is never woken up");

        reading.join().unwrap();
        assert!(buffers.len() <= 2);
    }

    /// What a stream over a `DoubleBuffer` is: a chunk goes as a `Bytes`
    struct ReadByChunks(DoubleBufferReader);

    #[async_trait::async_trait]
    impl AsyncBytesStream<String> for ReadByChunks {
        async fn get_next(&self) -> Result<Option<Bytes>, String> {
            Ok(self.0.get_next().await.map(DoubleBufferChunk::into_bytes))
        }

        fn get_size(&self) -> Option<usize> {
            None
        }
    }

    async fn read_lines(reader: &mut BufferedReader<String, ReadByChunks>) -> Vec<String> {
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
    fn it_is_parsed_through_a_buffered_reader_whatever_the_size_of_the_buffers() {
        const SRC: &str =
            "{\"id\":1}\n{\"id\":22,\"name\":\"some name\"}\n\n{\"id\":333}\nnot complete";

        rt().block_on(async {
            for buffer_size in 1..=SRC.len() + 1 {
                let (writer, reader) = DoubleBuffer::new(buffer_size);

                let reading = tokio::spawn(async move {
                    let mut buffers = HashSet::new();

                    for piece in SRC.as_bytes().chunks(buffer_size) {
                        let mut buffer = writer.get_buffer_to_read().await.unwrap();
                        buffers.insert(buffer.as_ptr() as usize);

                        buffer[..piece.len()].copy_from_slice(piece);
                        buffer.send(piece.len());
                    }

                    buffers.len()
                });

                let mut parser = BufferedReader::new(ReadByChunks(reader));

                // A line is longer than a buffer, and it is cut wherever it happens
                // to be - the reader holds a chunk while it asks for the next one
                let lines = tokio::time::timeout(Duration::from_secs(10), read_lines(&mut parser))
                    .await
                    .unwrap_or_else(|_| panic!("it hangs, buffer_size: {}", buffer_size));

                assert_eq!(
                    lines,
                    [
                        "{\"id\":1}",
                        "{\"id\":22,\"name\":\"some name\"}",
                        "",
                        "{\"id\":333}"
                    ],
                    "buffer_size: {}",
                    buffer_size
                );
                assert_eq!(parser.as_slice(), b"not complete");
                assert!(reading.await.unwrap() <= 2);
            }
        });
    }
}
