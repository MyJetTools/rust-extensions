use std::collections::VecDeque;
use std::future::Future;
use std::ops::{Deref, DerefMut, Range};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use parking_lot::Mutex;

use crate::AsyncBytesStream;

const BUFFERS: usize = 2;

/// Two buffers between the one who reads a stream and the one who processes it -
/// while one is processed, the next one is read into the other.
///
/// `new()` gives the two ends:
/// - [`DoubleBufferWriter`] is for the one who reads: `get_buffer_to_read()` gives a
///   buffer to read into, `send()` of that buffer hands over what is read -
///   `send_range()` a part of it, with what frames the data cut off - and
///   `finish()` is the end of the stream;
/// - [`DoubleBufferReader`] is for the one who processes: `get_next()` gives what is
///   read - a [`DoubleBufferChunk`], a `&[u8]` through `Deref`. Dropping the chunk
///   says that it is processed - all of it: the whole buffer is free to be read
///   into again.
///
/// Both wait. `get_buffer_to_read()` waits until one of the two buffers is free, so
/// the one who reads is never more than two buffers ahead of the one who processes.
/// `get_next()` waits until something is read.
///
/// Both learn that the other end is gone - [`DoubleBufferError::Disconnected`]: a
/// writer dropped with no `finish()` is a stream which is not read to its end, and a
/// reader dropped is nobody to process what is read.
///
/// The buffers are `Vec<u8>`s, each allocated once - when it is taken for the first
/// time - and nothing is copied on the way.
pub struct DoubleBuffer;

impl DoubleBuffer {
    /// The two ends. Each of the two buffers is `buffer_size` bytes.
    ///
    /// Panics when `buffer_size` is 0: nothing could be read into such a buffer, and
    /// a read of nothing is how the end of a stream is told.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(buffer_size: usize) -> (DoubleBufferWriter, DoubleBufferReader) {
        assert!(buffer_size > 0, "DoubleBuffer::new: the size of a buffer is 0");

        let inner = Arc::new(DoubleBufferInner {
            buffer_size,
            state: Mutex::new(State {
                free: Vec::with_capacity(BUFFERS),
                not_created: BUFFERS,
                read: VecDeque::with_capacity(BUFFERS),
                writer: WriterState::Writing,
                reader_is_dropped: false,
                waiting_for_buffer: Waiters::default(),
                waiting_for_read: Waiters::default(),
                waiting_for_close: Waiters::default(),
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

/// The other end of a [`DoubleBuffer`] is gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoubleBufferError {
    /// For the one who processes: the writer is dropped with no `finish()` - the
    /// connection is gone, the reading has failed, the task is cancelled. What came
    /// before it is not the whole stream.
    ///
    /// For the one who reads: the reader is dropped - nobody is going to process
    /// what is read.
    Disconnected,
}

impl std::fmt::Display for DoubleBufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disconnected => f.write_str("The other end of the double buffer is gone"),
        }
    }
}

impl std::error::Error for DoubleBufferError {}

struct DoubleBufferInner {
    buffer_size: usize,
    state: Mutex<State>,
}

/// How far the writer has got
#[derive(Clone, Copy, PartialEq, Eq)]
enum WriterState {
    Writing,
    /// `finish()` - all there is, is sent
    Finished,
    /// Dropped with no `finish()` - the stream is cut short
    Dropped,
}

struct State {
    /// The buffers nobody reads into and nobody processes
    free: Vec<Vec<u8>>,
    /// How many of the two buffers are not allocated yet
    not_created: usize,
    /// What is read and not taken to be processed yet - a buffer and where in it the
    /// bytes to process are, in the order they were sent
    read: VecDeque<(Vec<u8>, Range<usize>)>,
    writer: WriterState,
    reader_is_dropped: bool,
    /// The waits for a buffer to read into
    waiting_for_buffer: Waiters,
    /// The waits for something to process
    waiting_for_read: Waiters,
    /// The waits for the reader to be dropped
    waiting_for_close: Waiters,
}

impl State {
    /// A buffer is free again. While both ends are there it is kept for the next
    /// read, and those who wait for a buffer are to be woken up. With an end gone
    /// nobody is going to read into it, and it is let go.
    fn free(&mut self, buffer: Vec<u8>) -> Wake {
        if self.writer != WriterState::Writing || self.reader_is_dropped {
            return Wake::Nobody;
        }

        self.free.push(buffer);
        self.waiting_for_buffer.take()
    }

    /// A buffer to read into, `None` while both are taken. `Some(Ok(None))` is the
    /// one which is not allocated yet - whoever has asked for it allocates it.
    fn take_buffer(&mut self) -> Option<Result<Option<Vec<u8>>, DoubleBufferError>> {
        if self.reader_is_dropped {
            return Some(Err(DoubleBufferError::Disconnected));
        }

        if let Some(buffer) = self.free.pop() {
            return Some(Ok(Some(buffer)));
        }

        if self.not_created > 0 {
            self.not_created -= 1;
            return Some(Ok(None));
        }

        None
    }

    /// What is read next, `None` while nothing is. `Some(Ok(None))` is the end of
    /// the stream.
    #[allow(clippy::type_complexity)]
    fn take_read(&mut self) -> Option<Result<Option<(Vec<u8>, Range<usize>)>, DoubleBufferError>> {
        if let Some(read) = self.read.pop_front() {
            return Some(Ok(Some(read)));
        }

        match self.writer {
            WriterState::Writing => None,
            WriterState::Finished => Some(Ok(None)),
            WriterState::Dropped => Some(Err(DoubleBufferError::Disconnected)),
        }
    }

    fn closed(&mut self) -> Option<()> {
        self.reader_is_dropped.then_some(())
    }

    fn buffer_waiters(&mut self) -> &mut Waiters {
        &mut self.waiting_for_buffer
    }

    fn read_waiters(&mut self) -> &mut Waiters {
        &mut self.waiting_for_read
    }

    fn close_waiters(&mut self) -> &mut Waiters {
        &mut self.waiting_for_close
    }
}

/// The end of a [`DoubleBuffer`] for the one who reads the stream.
///
/// `finish()` is the end of the stream. A writer dropped with no `finish()` - the
/// connection is gone, the task has failed or is cancelled - is a stream cut short:
/// the reader gets what was sent, and [`DoubleBufferError::Disconnected`] after it.
pub struct DoubleBufferWriter {
    inner: Arc<DoubleBufferInner>,
}

impl DoubleBufferWriter {
    /// A buffer to read the stream into. It waits until one of the two is free -
    /// that is, until the chunk read into it before is dropped by the one who
    /// processes it.
    ///
    /// `Err(Disconnected)` - the reader is dropped: nobody is going to process what
    /// is read.
    pub async fn get_buffer_to_read(&self) -> Result<BufferToRead<'_>, DoubleBufferError> {
        let buffer = Waiting::new(&self.inner, State::take_buffer, State::buffer_waiters).await?;

        // A buffer taken for the first time is allocated with the lock released
        let buffer = buffer.unwrap_or_else(|| vec![0; self.inner.buffer_size]);

        Ok(BufferToRead {
            buffer,
            writer: self,
        })
    }

    /// The end of the stream: the reader gets what was sent, and `None` after it.
    pub fn finish(self) {
        self.inner.state.lock().writer = WriterState::Finished;
    }

    /// `true` - the reader is dropped, and nobody is going to process what is read.
    pub fn is_closed(&self) -> bool {
        self.inner.state.lock().reader_is_dropped
    }

    /// Resolves once the reader is dropped. A read which may wait long - a socket
    /// which is silent - is raced against it, so the one who reads learns at once
    /// that nobody is going to process what it reads.
    pub async fn closed(&self) {
        Waiting::new(&self.inner, State::closed, State::close_waiters).await
    }
}

impl Drop for DoubleBufferWriter {
    fn drop(&mut self) {
        let (free, waiting) = {
            let mut state = self.inner.state.lock();

            if state.writer == WriterState::Writing {
                // No `finish()`: the stream is cut short
                state.writer = WriterState::Dropped;
            }

            (
                std::mem::take(&mut state.free),
                state.waiting_for_read.take(),
            )
        };

        // Nobody is going to read into them
        drop(free);
        waiting.wake();
    }
}

/// A buffer to read the stream into - all the `buffer_size` bytes of it, as a
/// `&mut [u8]`. What was read into it before is still there: it is not cleared.
///
/// `send()` hands over what is read, `send_range()` a part of it. A buffer dropped
/// with no `send()` - a read which was given up, say - is free to be taken again.
pub struct BufferToRead<'s> {
    /// Empty once it is sent
    buffer: Vec<u8>,
    writer: &'s DoubleBufferWriter,
}

impl BufferToRead<'_> {
    /// `size` bytes are read into the beginning of the buffer - they go to the reader,
    /// and one of its next `get_next()` gives them.
    ///
    /// With `size` of `0` there is nothing to process, and with the reader dropped
    /// there is nobody to process it: the buffer is just free again.
    ///
    /// Panics when the buffer is smaller than `size` bytes.
    pub fn send(self, size: usize) {
        assert!(
            size <= self.buffer.len(),
            "send: {} bytes are sent, and the buffer has {} of them",
            size,
            self.buffer.len()
        );

        self.hand_over(0..size);
    }

    /// The bytes to process are `data` of the buffer - what is read, with what frames
    /// it cut off: the head of a response before them, the size of a chunk, the
    /// separator after them. They go to the reader, and the chunk one of its next
    /// `get_next()` gives is those bytes and nothing else. The buffer is still free
    /// again as a whole once the chunk is dropped, and the next read into it is given
    /// all of it again.
    ///
    /// `data` has its end: the bytes behind what is read are what was read into the
    /// buffer before. `send_range(0..size)` is `send(size)`.
    ///
    /// With `data` empty there is nothing to process, and with the reader dropped
    /// there is nobody to process it: the buffer is just free again.
    ///
    /// Panics when `data` ends past the buffer, or begins after its end.
    pub fn send_range(self, data: Range<usize>) {
        assert!(
            data.start <= data.end && data.end <= self.buffer.len(),
            "send_range: {}..{} is sent, and the buffer has {} bytes",
            data.start,
            data.end,
            self.buffer.len()
        );

        self.hand_over(data);
    }

    fn hand_over(mut self, data: Range<usize>) {
        if data.is_empty() {
            return;
        }

        let buffer = std::mem::take(&mut self.buffer);
        let mut state = self.writer.inner.state.lock();

        if state.reader_is_dropped {
            // Nobody is going to process it - it is let go with the lock released
            drop(state);
            return;
        }

        state.read.push_back((buffer, data));
        let waiting = state.waiting_for_read.take();
        drop(state);

        waiting.wake();
    }
}

impl Deref for BufferToRead<'_> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.buffer
    }
}

impl DerefMut for BufferToRead<'_> {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }
}

impl Drop for BufferToRead<'_> {
    fn drop(&mut self) {
        // Not sent - there is nothing to process in it
        if !self.buffer.is_empty() {
            let buffer = std::mem::take(&mut self.buffer);
            let waiting = self.writer.inner.state.lock().free(buffer);
            waiting.wake();
        }
    }
}

/// The end of a [`DoubleBuffer`] for the one who processes the stream.
///
/// Dropping it stops the one who reads: every wait of the writer ends with
/// [`DoubleBufferError::Disconnected`].
pub struct DoubleBufferReader {
    inner: Arc<DoubleBufferInner>,
}

impl DoubleBufferReader {
    /// What is read - the chunks come in the order they were sent. It waits until
    /// there is one.
    ///
    /// A chunk holds one of the two buffers, and dropping it says that it is
    /// processed. While both chunks are held nothing is read, so a `get_next()`
    /// which waits then waits forever: a chunk is processed and dropped, and what is
    /// needed later is copied out of it.
    ///
    /// - `Ok(None)` - the writer has called `finish()`, and all it has sent is given.
    /// - `Err(Disconnected)` - the writer is dropped with no `finish()`: the stream is
    ///   cut short. It comes after all that was sent before, and every call after it
    ///   gives it again.
    pub async fn get_next(&self) -> Result<Option<DoubleBufferChunk>, DoubleBufferError> {
        let read = Waiting::new(&self.inner, State::take_read, State::read_waiters).await?;

        Ok(read.map(|(buffer, data)| DoubleBufferChunk {
            buffer,
            data,
            inner: self.inner.clone(),
        }))
    }
}

/// The reader is a stream: its chunks are what `get_next()` gives, and it ends the
/// way the writer has ended it.
#[async_trait::async_trait]
impl AsyncBytesStream<DoubleBufferError> for DoubleBufferReader {
    type Chunk = DoubleBufferChunk;

    async fn get_next(&self) -> Result<Option<DoubleBufferChunk>, DoubleBufferError> {
        DoubleBufferReader::get_next(self).await
    }

    fn get_size(&self) -> Option<usize> {
        None
    }
}

impl Drop for DoubleBufferReader {
    fn drop(&mut self) {
        let (read, free, waiting_for_buffer, waiting_for_close) = {
            let mut state = self.inner.state.lock();
            state.reader_is_dropped = true;

            (
                std::mem::take(&mut state.read),
                std::mem::take(&mut state.free),
                state.waiting_for_buffer.take(),
                state.waiting_for_close.take(),
            )
        };

        // Nobody is going to process them, or to read into them
        drop((read, free));

        waiting_for_buffer.wake();
        waiting_for_close.wake();
    }
}

/// What is read - the bytes to process, as a `&[u8]` through `Deref`: what `send()`
/// or `send_range()` has handed over, and nothing else of the buffer.
///
/// It holds one of the two buffers. Dropping it says that it is processed - all of
/// it: the buffer is free to be read into again.
pub struct DoubleBufferChunk {
    buffer: Vec<u8>,
    /// Where the bytes to process are in `buffer`
    data: Range<usize>,
    inner: Arc<DoubleBufferInner>,
}

impl DoubleBufferChunk {
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer[self.data.clone()]
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
        let buffer = std::mem::take(&mut self.buffer);
        let waiting = self.inner.state.lock().free(buffer);
        waiting.wake();
    }
}

/// Waits in one of the lists of [`State`] until `check` gives what it waits for.
///
/// Its waker is kept under a key: polled again, the future replaces it, and dropped,
/// it takes it out. So a wait which is given up takes no wake-up away from the ones
/// which are not, and a future polled again and again does not make the list grow.
struct Waiting<'s, T> {
    inner: &'s DoubleBufferInner,
    key: Option<u64>,
    check: fn(&mut State) -> Option<T>,
    waiters: fn(&mut State) -> &mut Waiters,
}

impl<'s, T> Waiting<'s, T> {
    fn new(
        inner: &'s DoubleBufferInner,
        check: fn(&mut State) -> Option<T>,
        waiters: fn(&mut State) -> &mut Waiters,
    ) -> Self {
        Self {
            inner,
            key: None,
            check,
            waiters,
        }
    }
}

impl<T> Future for Waiting<'_, T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let this = self.get_mut();
        let mut state = this.inner.state.lock();

        match (this.check)(&mut state) {
            Some(result) => {
                if let Some(key) = this.key.take() {
                    (this.waiters)(&mut state).remove(key);
                }

                Poll::Ready(result)
            }
            None => {
                this.key = Some((this.waiters)(&mut state).register(this.key, cx.waker()));
                Poll::Pending
            }
        }
    }
}

impl<T> Drop for Waiting<'_, T> {
    fn drop(&mut self) {
        if let Some(key) = self.key {
            (self.waiters)(&mut self.inner.state.lock()).remove(key);
        }
    }
}

/// The wakers of the futures which wait for the same thing, each under the key of
/// its future
#[derive(Default)]
struct Waiters {
    entries: Vec<(u64, Waker)>,
    last_key: u64,
}

impl Waiters {
    /// Registers the waker of a future: `key` is what the future got the time before -
    /// `None` the first time - and the key to keep is returned
    fn register(&mut self, key: Option<u64>, waker: &Waker) -> u64 {
        let key = match key {
            Some(key) => {
                if let Some((_, registered)) = self.entries.iter_mut().find(|(k, _)| *k == key) {
                    registered.clone_from(waker);
                    return key;
                }

                // It was woken up, and somebody else has taken what it waits for
                key
            }
            None => {
                self.last_key += 1;
                self.last_key
            }
        };

        self.entries.push((key, waker.clone()));
        key
    }

    fn remove(&mut self, key: u64) {
        if let Some(position) = self.entries.iter().position(|(k, _)| *k == key) {
            self.entries.swap_remove(position);
        }
    }

    /// Takes the wakers out, to be woken with the lock released. One waker - the
    /// usual case - is taken alone, so the list keeps its room and the next wait
    /// allocates nothing.
    fn take(&mut self) -> Wake {
        if self.entries.len() > 1 {
            return Wake::All(std::mem::take(&mut self.entries));
        }

        match self.entries.pop() {
            Some((_, waker)) => Wake::One(waker),
            None => Wake::Nobody,
        }
    }
}

/// Wakers taken out of [`State`]. They are woken with the lock released: a waker may
/// run or drop a task right away, and a task may hold a buffer - which comes back
/// through the same lock.
#[must_use]
enum Wake {
    Nobody,
    One(Waker),
    All(Vec<(u64, Waker)>),
}

impl Wake {
    fn wake(self) {
        match self {
            Self::Nobody => {}
            Self::One(waker) => waker.wake(),
            Self::All(waiting) => {
                for (_, waker) in waiting {
                    waker.wake();
                }
            }
        }
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

    use super::{
        DoubleBuffer, DoubleBufferError, DoubleBufferInner, DoubleBufferReader,
        DoubleBufferWriter,
    };
    use crate::{AsyncBytesStream, BufferedReader};

    /// Counts how many times the one who waits is woken up. One which takes the lock
    /// of the double buffer when it is woken would wait for it forever if it were
    /// woken with the lock held.
    #[derive(Default)]
    struct WakeUps {
        count: AtomicUsize,
        takes_the_lock: Option<Arc<DoubleBufferInner>>,
    }

    impl WakeUps {
        fn new() -> Arc<Self> {
            Arc::new(Self::default())
        }

        fn taking_the_lock_of(writer: &DoubleBufferWriter) -> Arc<Self> {
            Arc::new(Self {
                count: AtomicUsize::new(0),
                takes_the_lock: Some(writer.inner.clone()),
            })
        }

        fn count(&self) -> usize {
            self.count.load(Ordering::Relaxed)
        }
    }

    impl Wake for WakeUps {
        fn wake(self: Arc<Self>) {
            if let Some(inner) = &self.takes_the_lock {
                drop(inner.state.lock());
            }

            self.count.fetch_add(1, Ordering::Relaxed);
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

    /// The next chunk, processed at once: a copy of it
    fn next(reader: &DoubleBufferReader) -> Result<Option<Vec<u8>>, DoubleBufferError> {
        now(reader.get_next()).map(|chunk| chunk.map(|chunk| chunk.to_vec()))
    }

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
    }

    #[test]
    fn what_is_read_comes_to_the_one_who_processes() {
        let (writer, reader) = DoubleBuffer::new(8);

        let mut buffer = now(writer.get_buffer_to_read()).unwrap();
        assert_eq!(buffer.len(), 8);
        let buffer_ptr = buffer.as_ptr();

        buffer[..5].copy_from_slice(b"Hello");
        buffer.send(5);

        let chunk = now(reader.get_next()).unwrap().unwrap();
        assert_eq!(&*chunk, b"Hello");
        // The very same buffer - nothing is copied
        assert_eq!(chunk.as_ptr(), buffer_ptr);
    }

    #[test]
    fn chunks_come_in_the_order_they_are_sent() {
        let (writer, reader) = DoubleBuffer::new(4);

        send(&writer, b"ab");
        send(&writer, b"cd");

        assert_eq!(next(&reader), Ok(Some(b"ab".to_vec())));
        assert_eq!(next(&reader), Ok(Some(b"cd".to_vec())));
    }

    #[test]
    fn a_buffer_is_free_once_its_chunk_is_dropped() {
        let (writer, reader) = DoubleBuffer::new(4);

        send(&writer, b"ab");
        send(&writer, b"cd");

        let wake_ups = WakeUps::new();
        let mut third = pin!(writer.get_buffer_to_read());
        assert!(poll(third.as_mut(), &wake_ups).is_pending());

        // It is taken - and it is being processed
        let ab = now(reader.get_next()).unwrap().unwrap();
        let ab_ptr = ab.as_ptr();
        assert!(poll(third.as_mut(), &wake_ups).is_pending());
        assert_eq!(wake_ups.count(), 0);

        // It is processed
        drop(ab);
        // Polled twice while it waited - and woken up once
        assert_eq!(wake_ups.count(), 1);

        let Poll::Ready(Ok(buffer)) = poll(third.as_mut(), &wake_ups) else {
            panic!("it waits");
        };
        assert_eq!(buffer.as_ptr(), ab_ptr);
        // What was read into it before is still there - it is not cleared
        assert_eq!(&buffer[..2], b"ab");
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

        let Poll::Ready(Ok(Some(chunk))) = poll(next.as_mut(), &wake_ups) else {
            panic!("it waits");
        };
        assert_eq!(&*chunk, [7]);
    }

    #[test]
    fn a_buffer_which_is_not_sent_is_free_again() {
        let (writer, reader) = DoubleBuffer::new(4);

        let first = now(writer.get_buffer_to_read()).unwrap();
        let second = now(writer.get_buffer_to_read()).unwrap();
        let second_ptr = second.as_ptr();

        let wake_ups = WakeUps::new();
        let mut third = pin!(writer.get_buffer_to_read());
        assert!(poll(third.as_mut(), &wake_ups).is_pending());

        // A read which was given up
        drop(second);
        assert_eq!(wake_ups.count(), 1);

        let Poll::Ready(Ok(again)) = poll(third.as_mut(), &wake_ups) else {
            panic!("it waits");
        };
        assert_eq!(again.as_ptr(), second_ptr);
        assert_ne!(again.as_ptr(), first.as_ptr());

        // Nothing has come to be processed
        assert!(poll(pin!(reader.get_next()), &WakeUps::new()).is_pending());
    }

    #[test]
    fn nothing_read_is_nothing_to_process() {
        let (writer, reader) = DoubleBuffer::new(4);

        now(writer.get_buffer_to_read()).unwrap().send(0);
        now(writer.get_buffer_to_read()).unwrap().send(0);

        // Both buffers are free again
        let _first = now(writer.get_buffer_to_read()).unwrap();
        let _second = now(writer.get_buffer_to_read()).unwrap();

        assert!(poll(pin!(reader.get_next()), &WakeUps::new()).is_pending());
    }

    #[test]
    fn send_range_hands_over_the_data_with_what_frames_it_cut_off() {
        let (writer, reader) = DoubleBuffer::new(16);

        // A head before the data
        let mut buffer = now(writer.get_buffer_to_read()).unwrap();
        let head_ptr = buffer.as_ptr();
        buffer[..13].copy_from_slice(b"HEAD\r\n\r\nHello");
        buffer.send_range(8..13);

        // The size of a chunk before the data, the separator after it
        let mut buffer = now(writer.get_buffer_to_read()).unwrap();
        let chunk_ptr = buffer.as_ptr();
        buffer[..10].copy_from_slice(b"5\r\nWorld\r\n");
        buffer.send_range(3..8);

        let hello = now(reader.get_next()).unwrap().unwrap();
        assert_eq!(&*hello, b"Hello");
        assert_eq!(hello.as_slice(), b"Hello");
        // The very same buffer - nothing is copied
        assert_eq!(hello.as_ptr(), head_ptr.wrapping_add(8));

        let world = now(reader.get_next()).unwrap().unwrap();
        assert_eq!(&*world, b"World");
        assert_eq!(world.as_ptr(), chunk_ptr.wrapping_add(3));
    }

    #[test]
    fn a_buffer_a_range_was_sent_from_is_read_into_as_a_whole_again() {
        let (writer, reader) = DoubleBuffer::new(8);

        let mut buffer = now(writer.get_buffer_to_read()).unwrap();
        let buffer_ptr = buffer.as_ptr();
        buffer[..5].copy_from_slice(b"HEADx");
        buffer.send_range(4..5);

        drop(now(reader.get_next()).unwrap().unwrap());

        // The same buffer, all of it - and what is sent of it now begins where it is
        // sent from
        let mut buffer = now(writer.get_buffer_to_read()).unwrap();
        assert_eq!(buffer.as_ptr(), buffer_ptr);
        assert_eq!(buffer.len(), 8);

        buffer[..2].copy_from_slice(b"ab");
        buffer.send(2);

        assert_eq!(next(&reader), Ok(Some(b"ab".to_vec())));
    }

    #[test]
    fn an_empty_range_is_nothing_to_process() {
        let (writer, reader) = DoubleBuffer::new(4);

        now(writer.get_buffer_to_read()).unwrap().send_range(0..0);
        now(writer.get_buffer_to_read()).unwrap().send_range(4..4);

        // Both buffers are free again
        let _first = now(writer.get_buffer_to_read()).unwrap();
        let _second = now(writer.get_buffer_to_read()).unwrap();

        assert!(poll(pin!(reader.get_next()), &WakeUps::new()).is_pending());
    }

    #[test]
    fn a_stream_of_ranges_is_the_data_and_nothing_else() {
        let (writer, reader) = DoubleBuffer::new(16);

        for (framed, data) in [(&b"HEAD\r\n\r\nHel"[..], 8..11), (b"2\r\nlo\r\n", 3..5)] {
            let mut buffer = now(writer.get_buffer_to_read()).unwrap();
            buffer[..framed.len()].copy_from_slice(framed);
            buffer.send_range(data);
        }

        writer.finish();

        assert_eq!(now(reader.into_vec()), Ok(b"Hello".to_vec()));
    }

    #[test]
    fn finish_is_the_end_of_the_stream() {
        let (writer, reader) = DoubleBuffer::new(4);

        send(&writer, b"ab");
        writer.finish();

        // What was sent before still comes, and the end after it
        assert_eq!(next(&reader), Ok(Some(b"ab".to_vec())));
        assert_eq!(next(&reader), Ok(None));
        assert_eq!(next(&reader), Ok(None));
    }

    #[test]
    fn a_writer_dropped_with_no_finish_cuts_the_stream_short() {
        let (writer, reader) = DoubleBuffer::new(4);

        send(&writer, b"ab");
        // The connection is gone before the end of the stream
        drop(writer);

        // What was sent before still comes
        assert_eq!(next(&reader), Ok(Some(b"ab".to_vec())));
        // And then it is known that it was not all of it
        assert_eq!(next(&reader), Err(DoubleBufferError::Disconnected));
        assert_eq!(next(&reader), Err(DoubleBufferError::Disconnected));
    }

    #[test]
    fn the_reader_which_waits_learns_how_the_stream_has_ended() {
        {
            let (writer, reader) = DoubleBuffer::new(4);

            let wake_ups = WakeUps::new();
            let mut next = pin!(reader.get_next());
            assert!(poll(next.as_mut(), &wake_ups).is_pending());

            writer.finish();
            assert_eq!(wake_ups.count(), 1);
            assert!(matches!(poll(next.as_mut(), &wake_ups), Poll::Ready(Ok(None))));
        }

        {
            let (writer, reader) = DoubleBuffer::new(4);

            let wake_ups = WakeUps::new();
            let mut next = pin!(reader.get_next());
            assert!(poll(next.as_mut(), &wake_ups).is_pending());

            drop(writer);
            assert_eq!(wake_ups.count(), 1);
            assert!(matches!(
                poll(next.as_mut(), &wake_ups),
                Poll::Ready(Err(DoubleBufferError::Disconnected))
            ));
        }
    }

    #[test]
    fn a_dropped_reader_ends_every_wait_of_the_writer() {
        let (writer, reader) = DoubleBuffer::new(4);

        send(&writer, b"ab");
        let taken = now(writer.get_buffer_to_read()).unwrap();

        let wake_ups = WakeUps::new();
        let mut third = pin!(writer.get_buffer_to_read());
        assert!(poll(third.as_mut(), &wake_ups).is_pending());
        let mut closed = pin!(writer.closed());
        assert!(poll(closed.as_mut(), &wake_ups).is_pending());
        assert!(!writer.is_closed());

        drop(reader);
        assert_eq!(wake_ups.count(), 2);
        assert!(matches!(
            poll(third.as_mut(), &wake_ups),
            Poll::Ready(Err(DoubleBufferError::Disconnected))
        ));
        assert_eq!(poll(closed.as_mut(), &wake_ups), Poll::Ready(()));
        assert!(writer.is_closed());

        // What was taken before goes nowhere
        taken.send(1);
        assert!(matches!(
            now(writer.get_buffer_to_read()),
            Err(DoubleBufferError::Disconnected)
        ));
    }

    #[test]
    fn a_chunk_outlives_both_ends() {
        let (writer, reader) = DoubleBuffer::new(4);

        send(&writer, b"ab");
        let chunk = now(reader.get_next()).unwrap().unwrap();

        drop(writer);
        drop(reader);

        assert_eq!(&*chunk, b"ab");
    }

    #[test]
    fn the_buffers_are_let_go_once_an_end_is_gone() {
        {
            let (writer, reader) = DoubleBuffer::new(4);

            send(&writer, b"ab");
            let chunk = now(reader.get_next()).unwrap().unwrap();
            let taken = now(writer.get_buffer_to_read()).unwrap();

            // The reader is dropped: what is sent, and what comes back, is not kept
            drop(reader);
            drop(chunk);
            drop(taken);

            let state = writer.inner.state.lock();
            assert!(state.free.is_empty());
            assert!(state.read.is_empty());
        }

        {
            let (writer, reader) = DoubleBuffer::new(4);

            send(&writer, b"ab");
            send(&writer, b"cd");
            writer.finish();

            // The writer is gone: the buffers which are processed are not kept
            assert_eq!(next(&reader), Ok(Some(b"ab".to_vec())));
            assert_eq!(next(&reader), Ok(Some(b"cd".to_vec())));
            assert_eq!(next(&reader), Ok(None));

            assert!(reader.inner.state.lock().free.is_empty());
        }
    }

    #[test]
    fn every_waker_is_woken_with_the_lock_released() {
        let (done, wait_until_done) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            // send() wakes the reader up
            {
                let (writer, reader) = DoubleBuffer::new(4);
                let wake_ups = WakeUps::taking_the_lock_of(&writer);

                let mut next = pin!(reader.get_next());
                assert!(poll(next.as_mut(), &wake_ups).is_pending());

                send(&writer, b"a");
                assert_eq!(wake_ups.count(), 1);
            }

            // The writer which is dropped wakes the reader up
            {
                let (writer, reader) = DoubleBuffer::new(4);
                let wake_ups = WakeUps::taking_the_lock_of(&writer);

                let mut next = pin!(reader.get_next());
                assert!(poll(next.as_mut(), &wake_ups).is_pending());

                drop(writer);
                assert_eq!(wake_ups.count(), 1);
            }

            // A chunk which is processed wakes the writer up
            {
                let (writer, reader) = DoubleBuffer::new(4);
                let wake_ups = WakeUps::taking_the_lock_of(&writer);

                send(&writer, b"a");
                send(&writer, b"b");

                let mut third = pin!(writer.get_buffer_to_read());
                assert!(poll(third.as_mut(), &wake_ups).is_pending());

                next(&reader).unwrap();
                assert_eq!(wake_ups.count(), 1);
            }

            // A buffer which is not sent wakes the writer up
            {
                let (writer, _reader) = DoubleBuffer::new(4);
                let wake_ups = WakeUps::taking_the_lock_of(&writer);

                let first = now(writer.get_buffer_to_read()).unwrap();
                let _second = now(writer.get_buffer_to_read()).unwrap();

                let mut third = pin!(writer.get_buffer_to_read());
                assert!(poll(third.as_mut(), &wake_ups).is_pending());

                drop(first);
                assert_eq!(wake_ups.count(), 1);
            }

            // The reader which is dropped wakes up every wait of the writer
            {
                let (writer, reader) = DoubleBuffer::new(4);
                let wake_ups = WakeUps::taking_the_lock_of(&writer);

                let _first = now(writer.get_buffer_to_read()).unwrap();
                let _second = now(writer.get_buffer_to_read()).unwrap();

                let mut third = pin!(writer.get_buffer_to_read());
                assert!(poll(third.as_mut(), &wake_ups).is_pending());
                let mut closed = pin!(writer.closed());
                assert!(poll(closed.as_mut(), &wake_ups).is_pending());

                drop(reader);
                assert_eq!(wake_ups.count(), 2);
            }

            done.send(()).unwrap();
        });

        wait_until_done
            .recv_timeout(Duration::from_secs(10))
            .expect("a waker is woken with the lock held, and waits for it forever");
    }

    #[test]
    fn a_wait_which_is_given_up_takes_no_wake_up_away() {
        for give_up_the_first in [true, false] {
            // Two waits for a buffer
            {
                let (writer, reader) = DoubleBuffer::new(4);

                send(&writer, b"ab");
                send(&writer, b"cd");

                let (first, second) = (WakeUps::new(), WakeUps::new());
                let mut a = Box::pin(writer.get_buffer_to_read());
                let mut b = Box::pin(writer.get_buffer_to_read());
                assert!(poll(a.as_mut(), &first).is_pending());
                assert!(poll(b.as_mut(), &second).is_pending());

                let (given_up, mut waits, wake_ups) = match give_up_the_first {
                    true => (a, b, &second),
                    false => (b, a, &first),
                };

                drop(given_up);
                assert_eq!(writer.inner.state.lock().waiting_for_buffer.entries.len(), 1);

                next(&reader).unwrap();

                assert_eq!(wake_ups.count(), 1);
                assert!(matches!(poll(waits.as_mut(), wake_ups), Poll::Ready(Ok(_))));
            }

            // Two waits for a chunk
            {
                let (writer, reader) = DoubleBuffer::new(4);

                let (first, second) = (WakeUps::new(), WakeUps::new());
                let mut a = Box::pin(reader.get_next());
                let mut b = Box::pin(reader.get_next());
                assert!(poll(a.as_mut(), &first).is_pending());
                assert!(poll(b.as_mut(), &second).is_pending());

                let (given_up, mut waits, wake_ups) = match give_up_the_first {
                    true => (a, b, &second),
                    false => (b, a, &first),
                };

                drop(given_up);
                assert_eq!(writer.inner.state.lock().waiting_for_read.entries.len(), 1);

                send(&writer, b"ab");

                assert_eq!(wake_ups.count(), 1);
                assert!(matches!(poll(waits.as_mut(), wake_ups), Poll::Ready(Ok(Some(_)))));
            }
        }
    }

    #[test]
    fn a_wait_polled_again_and_again_is_kept_once() {
        let (writer, reader) = DoubleBuffer::new(4);

        send(&writer, b"ab");
        send(&writer, b"cd");

        // Every poll with a waker of its own - the way `will_wake()` may see them
        let wake_ups: Vec<_> = (0..100).map(|_| WakeUps::new()).collect();

        let mut third = pin!(writer.get_buffer_to_read());
        for wake_ups in &wake_ups {
            assert!(poll(third.as_mut(), wake_ups).is_pending());
        }

        assert_eq!(writer.inner.state.lock().waiting_for_buffer.entries.len(), 1);

        next(&reader).unwrap();

        // The waker it was polled with last is woken up, and only that one
        assert_eq!(wake_ups[99].count(), 1);
        assert_eq!(wake_ups.iter().map(|w| w.count()).sum::<usize>(), 1);
    }

    #[test]
    fn the_one_who_waits_alone_leaves_the_room_of_the_list_to_the_next() {
        let (writer, reader) = DoubleBuffer::new(4);

        send(&writer, b"ab");
        send(&writer, b"cd");

        let mut third = pin!(writer.get_buffer_to_read());
        assert!(poll(third.as_mut(), &WakeUps::new()).is_pending());

        // "ab" is processed, and the one who waits is woken up
        next(&reader).unwrap();

        let state = writer.inner.state.lock();
        assert!(state.waiting_for_buffer.entries.is_empty());
        // The next wait allocates nothing
        assert!(state.waiting_for_buffer.entries.capacity() > 0);
    }

    #[test]
    fn a_wait_for_a_chunk_which_is_given_up_loses_nothing() {
        let (writer, reader) = DoubleBuffer::new(4);

        // It waits, and is given up - a `select!` which took the other branch
        assert!(poll(pin!(reader.get_next()), &WakeUps::new()).is_pending());

        send(&writer, b"ab");
        assert_eq!(next(&reader), Ok(Some(b"ab".to_vec())));
    }

    #[test]
    #[should_panic(expected = "DoubleBuffer::new: the size of a buffer is 0")]
    fn a_buffer_of_no_size_panics() {
        let _ = DoubleBuffer::new(0);
    }

    #[test]
    #[should_panic(expected = "send: 5 bytes are sent, and the buffer has 4 of them")]
    fn send_of_more_than_the_buffer_has_panics() {
        let (writer, _reader) = DoubleBuffer::new(4);

        now(writer.get_buffer_to_read()).unwrap().send(5);
    }

    #[test]
    #[should_panic(expected = "send_range: 2..5 is sent, and the buffer has 4 bytes")]
    fn send_range_past_the_end_of_the_buffer_panics() {
        let (writer, _reader) = DoubleBuffer::new(4);

        now(writer.get_buffer_to_read()).unwrap().send_range(2..5);
    }

    #[test]
    #[should_panic(expected = "send_range: 3..2 is sent, and the buffer has 4 bytes")]
    #[allow(clippy::reversed_empty_ranges)]
    fn send_range_which_begins_after_its_end_panics() {
        let (writer, _reader) = DoubleBuffer::new(4);

        now(writer.get_buffer_to_read()).unwrap().send_range(3..2);
    }

    #[test]
    fn the_ends_the_chunks_and_the_futures_go_to_other_tasks() {
        fn is_send<T: Send>(_: &T) {}
        fn is_send_sync_and_static<T: Send + Sync + 'static>(_: T) {}

        let (writer, reader) = DoubleBuffer::new(4);

        is_send(&writer.get_buffer_to_read());
        is_send(&writer.closed());
        is_send(&reader.get_next());

        send(&writer, b"ab");
        is_send_sync_and_static(now(reader.get_next()).unwrap().unwrap());

        is_send_sync_and_static(writer);
        is_send_sync_and_static(reader);
    }

    #[test]
    fn it_is_read_in_one_thread_and_processed_in_another() {
        const CHUNKS: u32 = 50_000;
        const BUFFER_SIZE: usize = 64;

        for finish in [true, false] {
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

                    if finish {
                        writer.finish();
                    }
                })
            });

            let processing = async {
                let mut buffers = HashSet::new();
                let mut no = 0u32;

                let end = loop {
                    let chunk = match reader.get_next().await {
                        Ok(Some(chunk)) => chunk,
                        Ok(None) => break Ok(()),
                        Err(err) => break Err(err),
                    };

                    buffers.insert(chunk.as_ptr() as usize);

                    // The one who processes is slow now and then - so both get to wait
                    if no % 5 == 0 {
                        tokio::task::yield_now().await;
                    }

                    // Whatever is read meanwhile, the chunk is what it was
                    assert_eq!(chunk.len(), 4 + no as usize % (BUFFER_SIZE - 3));
                    assert_eq!(chunk[..4], no.to_le_bytes());
                    assert!(chunk[4..].iter().all(|b| *b == no as u8));

                    no += 1;
                };

                (no, end, buffers.len())
            };

            let (processed, end, buffers) = rt()
                .block_on(async { tokio::time::timeout(Duration::from_secs(60), processing).await })
                .expect("somebody waits and is never woken up");

            reading.join().unwrap();

            assert_eq!(processed, CHUNKS);
            assert!(buffers <= 2);

            match finish {
                true => assert_eq!(end, Ok(())),
                false => assert_eq!(end, Err(DoubleBufferError::Disconnected)),
            }
        }
    }

    /// Reads the lines the way a parser does: takes what is complete, and asks for
    /// more when nothing is. Gives the lines, and how the stream has ended
    async fn read_lines(
        reader: &mut BufferedReader<DoubleBufferError, DoubleBufferReader>,
    ) -> (Vec<String>, Result<(), DoubleBufferError>) {
        let mut lines = Vec::new();

        loop {
            if let Some(end) = reader.as_slice().iter().position(|b| *b == b'\n') {
                lines.push(String::from_utf8(reader.as_slice()[..end].to_vec()).unwrap());
                reader.mark_as_read(end + 1);
                continue;
            }

            if !reader.read_mode() {
                return (lines, Ok(()));
            }

            if let Err(err) = reader.get_next().await {
                return (lines, Err(err));
            }
        }
    }

    #[test]
    fn it_is_parsed_through_a_buffered_reader_whatever_the_size_of_the_buffers() {
        const SRC: &str =
            "{\"id\":1}\n{\"id\":22,\"name\":\"some name\"}\n\n{\"id\":333}\nnot complete";

        rt().block_on(async {
            for buffer_size in 1..=SRC.len() + 1 {
                for finish in [true, false] {
                    let (writer, reader) = DoubleBuffer::new(buffer_size);

                    let reading = tokio::spawn(async move {
                        for piece in SRC.as_bytes().chunks(buffer_size) {
                            let mut buffer = writer.get_buffer_to_read().await.unwrap();
                            buffer[..piece.len()].copy_from_slice(piece);
                            buffer.send(piece.len());
                        }

                        if finish {
                            writer.finish();
                        }
                    });

                    let mut parser = BufferedReader::new(reader);

                    // A line is longer than a buffer, and it is cut wherever it happens
                    // to be
                    let (lines, end) =
                        tokio::time::timeout(Duration::from_secs(10), read_lines(&mut parser))
                            .await
                            .unwrap_or_else(|_| panic!("it hangs, buffer_size: {}", buffer_size));

                    reading.await.unwrap();

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

                    // A stream which is cut short is not taken for a whole one
                    match finish {
                        true => assert_eq!(end, Ok(())),
                        false => assert_eq!(end, Err(DoubleBufferError::Disconnected)),
                    }
                }
            }
        });
    }

    #[test]
    fn into_vec_reads_it_all_or_learns_that_it_is_cut_short() {
        rt().block_on(async {
            for finish in [true, false] {
                let (writer, reader) = DoubleBuffer::new(4);

                let reading = tokio::spawn(async move {
                    for piece in b"hello world".chunks(4) {
                        let mut buffer = writer.get_buffer_to_read().await.unwrap();
                        buffer[..piece.len()].copy_from_slice(piece);
                        buffer.send(piece.len());
                    }

                    if finish {
                        writer.finish();
                    }
                });

                let result = AsyncBytesStream::into_vec(&reader).await;
                reading.await.unwrap();

                match finish {
                    true => assert_eq!(result, Ok(b"hello world".to_vec())),
                    false => assert_eq!(result, Err(DoubleBufferError::Disconnected)),
                }
            }
        });
    }
}
