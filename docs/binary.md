# binary

Build and read byte payloads: integers, variable-size lengths, borrowed-or-owned buffers, byte search, cursors over bytes, byte streams read in chunks and parsed across them, two buffers between the task which reads and the task which parses, hex and base64.

## BinaryPayloadBuilder

Integers in little-endian, into a growing `Vec<u8>` or into a buffer you pass in. Both modes give the same bytes. `.into()` turns the builder into a `SliceOrVec<u8>` holding what was written. In slice mode that is the written part only. A write that does not fit into your buffer panics.

```rust
use rust_extensions::{BinaryPayloadBuilder, SliceOrVec};

let mut builder = BinaryPayloadBuilder::new_as_vec();
builder.write_u8(1);
builder.write_u16(0x0102);
builder.write_i32(-1);
builder.write_u64(7);
let bytes: SliceOrVec<u8> = builder.into();
assert_eq!(&bytes.as_slice()[..7], &[1, 0x02, 0x01, 0xff, 0xff, 0xff, 0xff]);

let mut buffer = [0u8; 16];
let mut builder = BinaryPayloadBuilder::new_as_slice(&mut buffer);
builder.write_u16(0x0102);
builder.write_i8(-1);
let written: SliceOrVec<u8> = builder.into();
assert_eq!(written.as_slice(), &[0x02, 0x01, 0xff]);
```

## UInt32VariableSize

A length or a counter in 1–4 bytes. The two top bits of the first byte hold the size:

| Value | Bytes |
| --- | --- |
| `< 64` | 1 |
| `< 16_384` | 2 |
| `< 4_194_303` | 3 |
| `< 1_073_741_823` | 4 (`new` panics on more) |

```rust
use rust_extensions::{ParseUInt32VariableSizeResult, UInt32VariableSize};

let mut out = Vec::new();
UInt32VariableSize::new(5).serialize(&mut out);
UInt32VariableSize::new(100).serialize(&mut out);
assert_eq!(out, vec![5, 0x40, 100]);

match UInt32VariableSize::from_slice(&out[1..]) {
    ParseUInt32VariableSizeResult::Ok { value, size } => {
        assert_eq!(value.get_value(), 100);
        assert_eq!(size, 2);
    }
    ParseUInt32VariableSizeResult::NotEnoughDataInBuffer(_) => unreachable!(),
}

// Wait for more bytes: the value needs 2, only 1 arrived
assert!(matches!(
    UInt32VariableSize::from_slice(&out[1..2]),
    ParseUInt32VariableSizeResult::NotEnoughDataInBuffer(2)
));
assert_eq!(UInt32VariableSize::from_slice(&out[..1]).unwrap().get_value(), 5);
```

`from_slice` panics on an empty slice.

## SliceOrVec

A borrowed `&[T]` or an owned `Vec<T>` behind one type. `SliceOrVecSeqReader` reads it through `std::io::Read + Seek`.

```rust
use std::io::{Read, Seek, SeekFrom};
use rust_extensions::{SliceOrVec, SliceOrVecSeqReader};

let borrowed: SliceOrVec<u8> = b"abc".as_slice().into();
let owned: SliceOrVec<u8> = vec![1u8, 2, 3].into();
let from_str: SliceOrVec<u8> = "abc".into();

assert!(borrowed.is_slice() && owned.is_vec());
assert_eq!(from_str.as_slice(), b"abc");
assert_eq!(owned.get_len(), 3);
assert_eq!(borrowed.into_vec(), b"abc".to_vec());

let mut reader: SliceOrVecSeqReader<u8> = from_str.into();
let mut buf = [0u8; 2];
assert_eq!(reader.read(&mut buf).unwrap(), 2);
reader.seek(SeekFrom::End(-1)).unwrap();
assert_eq!(reader.read(&mut buf).unwrap(), 1);
assert_eq!(buf[0], b'c');
```

## Searching bytes — SliceOfU8Ext

```rust
use rust_extensions::slice_of_u8_utils::SliceOfU8Ext;

let src = b"key: value\r\n";

assert_eq!(src.find_byte_pos(b':', 0), Some(3));
assert_eq!(src.find_sequence_pos(b"\r\n", 0), Some(10));
assert_eq!(src.find_pos_by_condition(4, |b| b != b' '), Some(5));
assert_eq!(src.find_byte_pos(b':', 4), None); // the search starts at the given position
```

## Cursors over bytes — array_of_bytes_iterator

`SliceIterator` (borrowed) and `VecIterator` (owned, can be extended) implement `ArrayOfBytesIterator`. It is a cursor that parsers move through `&self`.

```rust
use rust_extensions::array_of_bytes_iterator::{ArrayOfBytesIterator, SliceIterator, VecIterator};

let src = SliceIterator::from_str("GET /path HTTP/1.1");

assert_eq!(src.advance(3), Some(b"GET".as_slice())); // None, and no move, if fewer are left
assert_eq!(src.get_next().unwrap().value, b' ');

let start = src.get_pos();
let space = src.peek_and_find_sequence_pos_from_current_pos(b" ").unwrap();
src.set_pos(space);
assert_eq!(src.get_slice_to_current_pos(start), b"/path");

assert!(src.peek_sequence(1, |next| next == b" "));
assert_eq!(src.peek_value().unwrap().value, b' '); // peek does not move
assert_eq!(src.get_slice_to_end(space + 1), b"HTTP/1.1");

let mut stream = VecIterator::from_str("ab");
stream.extend(b"c");
stream.get_next();
stream.gc(); // drops what is already read, the position goes back to 0
assert_eq!(stream.get_src_slice(), b"bc");
```

With `with-tokio`, `FileIterator` walks a file through a window of `buffer_size` bytes. It implements the async twin of the cursor, `ArrayOfBytesIteratorAsync`.

```rust
use rust_extensions::array_of_bytes_iterator::{ArrayOfBytesIteratorAsync, FileIterator};

async fn count_lines(path: &str) -> std::io::Result<usize> {
    let mut src = FileIterator::new(path, 64 * 1024).await?;
    let mut lines = 0;

    while let Some(next) = src.get_next().await? {
        if next.value == b'\n' {
            lines += 1;
        }
    }

    Ok(lines)
}
```

## AsyncBytesStream — bytes read in chunks

A trait for a stream of bytes that arrives chunk by chunk: a file, a response body, a blob downloaded in parts. It needs no feature.

- `get_next()` returns `Ok(Some(chunk))` with the next chunk, and `Ok(None)` at the end of the data. A chunk gives its bytes as a `&[u8]` through `Deref`, whatever the stream holds them in — a `Vec<u8>`, a `Bytes`, a buffer of a `DoubleBuffer`. That is the `Chunk` type of the stream.
- `get_size()` returns the size of the whole stream in bytes, or `None` when it is not known before the stream is read.
- `into_vec()` is already implemented. It reads the stream to the end and returns everything as one `Vec<u8>`.

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use rust_extensions::AsyncBytesStream;

/// Hands its chunks over one by one
struct Chunks {
    chunks: Vec<&'static [u8]>,
    next_chunk: AtomicUsize,
}

#[async_trait::async_trait]
impl AsyncBytesStream<std::io::Error> for Chunks {
    // Whatever gives a `&[u8]` through `Deref`: a `Vec<u8>`, a `Bytes`, a `&'static [u8]`
    type Chunk = &'static [u8];

    async fn get_next(&self) -> std::io::Result<Option<&'static [u8]>> {
        let chunk_no = self.next_chunk.fetch_add(1, Ordering::Relaxed);
        // None once the chunks are over
        Ok(self.chunks.get(chunk_no).copied())
    }

    fn get_size(&self) -> Option<usize> {
        Some(self.chunks.iter().map(|chunk| chunk.len()).sum())
    }
}

let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();

rt.block_on(async {
    // Chunk by chunk...
    let src = Chunks { chunks: vec![b"hello ", b"world"], next_chunk: AtomicUsize::new(0) };
    let mut read = 0;
    while let Some(chunk) = src.get_next().await.unwrap() {
        read += chunk.len(); // what is needed later is copied out of the chunk
    }
    assert_eq!(read, 11);

    // ...or everything at once.
    let src = Chunks { chunks: vec![b"hello ", b"world"], next_chunk: AtomicUsize::new(0) };
    assert_eq!(src.get_size(), Some(11));
    assert_eq!(src.into_vec().await.unwrap(), b"hello world");
});
```

The contracts:

- **Dropping a chunk says that it is processed.** A stream which reads into buffers of its own reads into that one again. So a chunk is processed and dropped, and what is needed later is copied out of it. A chunk which is kept keeps its buffer: a `DoubleBuffer` stops reading while both of its chunks are held.
- **Everything takes `&self`.** The position lives in an atomic or behind a lock, and the stream works as `Arc<dyn AsyncBytesStream<TError, Chunk = ...> + Send + Sync>`, `into_vec()` included. An `Arc` of a stream is a stream itself, so it goes wherever one is taken.
- **`into_vec()` copies every chunk** into one `Vec`, allocated for `get_size()` bytes at once when the size is known. The size is only a hint for that: the stream is read until `None` whatever it says.
- **`into_vec()` reads what is left.** Called after some `get_next()`, it returns only the remaining bytes.
- **Errors.** `into_vec()` returns the first error, and the chunks read before it are dropped.

## BufferedReader — parsing a stream cut into chunks

A stream is cut into chunks wherever it happens to be, so a JSON, a line or a frame may begin in one chunk and end in the next. `BufferedReader` wraps an `AsyncBytesStream` and keeps what is read and not parsed yet in a buffer of its own, as one run of bytes. It needs no feature.

- `as_slice()` is what there is to parse.
- `mark_as_read(size)` drops `size` parsed bytes from the beginning.
- `get_next()` reads the next chunk in when there is not enough to parse. It returns all there is to parse — the bytes of `as_slice()`.
- `read_mode()` is `false` once the stream is read to its end.

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use rust_extensions::{AsyncBytesStream, BufferedReader};

struct Chunks {
    chunks: Vec<&'static [u8]>,
    next_chunk: AtomicUsize,
}

#[async_trait::async_trait]
impl AsyncBytesStream<std::io::Error> for Chunks {
    type Chunk = &'static [u8];

    async fn get_next(&self) -> std::io::Result<Option<&'static [u8]>> {
        let chunk_no = self.next_chunk.fetch_add(1, Ordering::Relaxed);
        Ok(self.chunks.get(chunk_no).copied())
    }

    fn get_size(&self) -> Option<usize> {
        None
    }
}

/// A JSON per line. `None` - the line is not complete yet.
fn next_json(src: &[u8]) -> Option<&[u8]> {
    let end = src.iter().position(|b| *b == b'\n')?;
    Some(&src[..end])
}

let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();

rt.block_on(async {
    // The chunks are cut in the middle of the lines
    let src = Chunks {
        chunks: vec![b"{\"id\":1}\n{\"i", b"d\":2}\n{\"id\"", b":3}\n"],
        next_chunk: AtomicUsize::new(0),
    };

    let mut reader = BufferedReader::new(src);
    let mut lines = Vec::new();

    loop {
        if let Some(json) = next_json(reader.as_slice()) {
            lines.push(String::from_utf8(json.to_vec()).unwrap());

            let size = json.len() + 1; // the line and its `\n`
            reader.mark_as_read(size);
            continue;
        }

        if !reader.read_mode() {
            break; // the stream is over - what is left is a line that was cut short
        }

        reader.get_next().await.unwrap(); // not enough to parse - the next chunk is read in
    }

    assert_eq!(lines, [r#"{"id":1}"#, r#"{"id":2}"#, r#"{"id":3}"#]);
    assert!(reader.as_slice().is_empty());
});
```

A chunk is copied into the reader's buffer as soon as it comes:

- **The reader holds none of the chunks of the stream.** Each one is dropped once it is copied, so a `DoubleBuffer` has its buffer back at once and reads into it while the copy is parsed.
- **The buffer is reused.** The room of what was marked as read is given back, and the buffer grows the way a `Vec` does — up to the longest run of bytes not parsed at once.
- **To parse with no copy at all**, read the stream itself: parse a chunk in place, and copy out only what is left of it.

The contracts:

- **`read_mode()` turns `false` one call after the last chunk.** The reader learns about the end when the stream returns `None`. That call returns what is left, and the stream is not asked again.
- **Chunks with no bytes are skipped.** After `get_next()` there is something new to parse, or `read_mode()` is `false`.
- **Errors.** `get_next()` returns the error of the stream as it is. What was read stays, and the next call asks the stream again.
- **`mark_as_read()` panics** when there are fewer than `size` bytes.
- **One parser.** Unlike the stream, the reader changes through `&mut self`.

## DoubleBuffer — read into one buffer while the other is processed

One task reads a stream, another one processes it. `DoubleBuffer` gives them two buffers: a buffer is read into and handed over, and while it is processed the next one is read into the other. Processed means all of it — the buffer is free to be read into again. The buffers are `Vec<u8>`s, allocated once, and nothing is copied. It needs no feature and no runtime.

`DoubleBuffer::new(buffer_size)` returns the two ends.

- **`DoubleBufferWriter`** is for the one who reads. `get_buffer_to_read()` gives a buffer — a `&mut [u8]` of `buffer_size` bytes — and waits until one of the two is free. `send(size)` of that buffer hands over the first `size` bytes of it, and `finish()` is the end of the stream.
- **`DoubleBufferReader`** is for the one who processes. `get_next()` gives what is read — a `DoubleBufferChunk`, a `&[u8]` through `Deref` — and waits until there is one. Dropping the chunk says that it is processed, and its buffer is free again. The reader is an `AsyncBytesStream` as well.

```rust
use rust_extensions::DoubleBuffer;
use tokio::io::AsyncReadExt;

let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();

rt.block_on(async {
    let (writer, reader) = DoubleBuffer::new(8);

    // The one who reads
    tokio::spawn(async move {
        let mut src: &[u8] = b"hello world, hello buffers"; // a socket, a file

        loop {
            // Err - the reader is dropped: nobody is going to process what is read
            let Ok(mut buffer) = writer.get_buffer_to_read().await else {
                return;
            };

            match src.read(&mut buffer).await {
                Ok(0) => {
                    // The stream is over. `buffer` borrows the writer - it goes first
                    drop(buffer);
                    writer.finish();
                    return;
                }
                Ok(size) => buffer.send(size),
                // The connection is gone: the writer is dropped with no finish(),
                // and the reader gets Err(Disconnected)
                Err(_) => return,
            }
        }
    });

    // The one who processes
    let mut result = Vec::new();

    while let Some(chunk) = reader.get_next().await.unwrap() {
        assert!(chunk.len() <= 8);
        result.extend_from_slice(&chunk); // what is needed later is copied
        // `chunk` is dropped here - it is processed, and its buffer is free again
    }

    assert_eq!(result, b"hello world, hello buffers");
});
```

A writer which is gone before `finish()` leaves a stream which is not read to its end — the connection is dropped, the read has failed, the task has panicked or is cancelled. The reader learns it:

```rust
use rust_extensions::{DoubleBuffer, DoubleBufferError};

let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();

rt.block_on(async {
    let (writer, reader) = DoubleBuffer::new(8);

    let mut buffer = writer.get_buffer_to_read().await.unwrap();
    buffer[..5].copy_from_slice(b"hello");
    buffer.send(5);

    // The connection is gone before the end of the stream
    drop(writer);

    // What was read before it still comes...
    let chunk = reader.get_next().await.unwrap();
    assert_eq!(chunk.as_deref(), Some(&b"hello"[..]));
    drop(chunk);

    // ...and then it is known that it was not all of it
    assert_eq!(reader.get_next().await.err(), Some(DoubleBufferError::Disconnected));
});
```

The contracts:

- **The one who reads is at most two buffers ahead.** With both buffers sent and not processed yet, `get_buffer_to_read()` waits.
- **A chunk is processed when it is dropped.** Process it and drop it, and copy out what is needed later. While both chunks are held nothing is read, so whoever holds both and waits for the next one waits forever. A `BufferedReader` over the reader copies each chunk as it comes and drops it at once.
- **A buffer dropped with no `send()` is free again.** That is what a read which was given up leaves — a `select!` which took the other branch, for example. `send(0)` does the same.
- **A buffer is not cleared.** What was read into it before is still there, so only the first `size` bytes of a read mean something.
- **The end of the stream is `finish()`.** The reader gets what was sent, then `Ok(None)`.
- **A writer dropped with no `finish()` is a stream cut short.** The reader gets what was sent, then `Err(DoubleBufferError::Disconnected)` on every call. A `get_next()` which waits is woken up with it.
- **A dropped reader stops the writer.** Every `get_buffer_to_read()`, waiting or not, gives `Err(DoubleBufferError::Disconnected)`, and what was sent and not taken is dropped. `closed()` resolves then: race it against a read which may wait long, such as a silent socket. `is_closed()` asks without waiting.
- **Memory.** A buffer is allocated when it is taken for the first time, and let go once an end is gone — that of a chunk which outlives the reader, too.
- **`new(0)` panics**, and so does `send()` of more than the buffer has.
- **Everything takes `&self`, and waiting needs no runtime** — it works under `tokio` and in a browser alike. A wait which is given up takes no wake-up away from the others.

## binary_search

`std` binary search for items that expose a `Copy + PartialOrd` key. It returns `Ok(index)` or `Err(insert_index)`.

```rust
use rust_extensions::binary_search::{binary_search, EntityWithBinarySearchKey};

struct Candle {
    time: i64,
}

impl EntityWithBinarySearchKey<i64> for Candle {
    fn get_key(&self) -> &i64 {
        &self.time
    }
}

let candles: Vec<Candle> = [10, 20, 30].into_iter().map(|time| Candle { time }).collect();

assert_eq!(binary_search(&candles, 20), Ok(1));
assert_eq!(binary_search(&candles, 25), Err(2));
assert_eq!(binary_search(&candles, 5), Err(0));
```

## vec_uninit!

A `Vec<u8>` of `len` bytes that are not zeroed, meant as a socket read buffer. Read only the part a read reported: `&buf[..n]`.

```rust
use rust_extensions::vec_uninit;

let mut buf = vec_uninit![64 * 1024];
// let n = socket.read(&mut buf).await?;
// process(&buf[..n]);
assert_eq!(buf.len(), 64 * 1024);
```

## hex (feature `hex`)

```rust
use rust_extensions::hex::{utils, HexArray, HexU16, HexU32, HexU8};

let hex = HexArray::from_slice(&[0xde, 0xad]);
assert_eq!(hex.as_str(), "dead");
assert_eq!(HexArray::from_slice_uppercase(&[0xde, 0xad]).as_str(), "DEAD");
assert_eq!(hex.to_bytes(), vec![0xde, 0xad]);

let parsed: HexArray = "beef".into(); // panics on a non-hex char
assert_eq!(parsed.to_bytes(), vec![0xbe, 0xef]);

assert_eq!(HexU8::new(0x0f).as_str(), "0f");
assert_eq!(HexU8::try_from("f").unwrap().to_u8(), 0x0f);
assert_eq!(HexU16::new(0x1234).as_str(), "1234");
assert_eq!(HexU32::new(0xff).as_str(), "000000ff");

assert_eq!(utils::array_of_bytes_to_hex(&[1, 255]), "01ff");
assert_eq!(utils::hex_array_to_bytes("01ff"), vec![1, 255]);
```

## base64 (feature `base64`)

Standard alphabet, with padding.

```rust
use rust_extensions::base64::{FromBase64, IntoBase64};

let encoded = b"hello".as_slice().into_base64();
assert_eq!(encoded, "aGVsbG8=");
assert_eq!(encoded.as_str().from_base64().unwrap(), b"hello");
assert!("not base64!".from_base64().is_err());
```
