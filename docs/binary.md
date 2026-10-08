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

- `get_next()` returns `Ok(Some(chunk))` with the next chunk, and `Ok(None)` at the end of the data. A chunk is a `Bytes` of the `bytes` crate, which is re-exported as `rust_extensions::bytes`.
- `get_size()` returns the size of the whole stream in bytes, or `None` when it is not known before the stream is read.
- `into_vec()` is already implemented. It reads the stream to the end and returns everything as one `Vec<u8>`.

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use rust_extensions::bytes::Bytes;
use rust_extensions::AsyncBytesStream;

struct Chunks {
    chunks: Vec<Bytes>,
    next_chunk: AtomicUsize,
}

impl Chunks {
    fn new(chunks: Vec<Vec<u8>>) -> Self {
        // A `Vec<u8>` becomes a chunk with no copy
        let chunks = chunks.into_iter().map(Bytes::from).collect();
        Self { chunks, next_chunk: AtomicUsize::new(0) }
    }
}

#[async_trait::async_trait]
impl AsyncBytesStream<std::io::Error> for Chunks {
    async fn get_next(&self) -> std::io::Result<Option<Bytes>> {
        let chunk_no = self.next_chunk.fetch_add(1, Ordering::Relaxed);
        // None once the chunks are over. Cloning a `Bytes` copies no data.
        Ok(self.chunks.get(chunk_no).cloned())
    }

    fn get_size(&self) -> Option<usize> {
        Some(self.chunks.iter().map(|chunk| chunk.len()).sum())
    }
}

let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();

rt.block_on(async {
    // Chunk by chunk...
    let src = Chunks::new(vec![b"hello ".to_vec(), b"world".to_vec()]);
    let mut chunks = 0;
    while let Some(_chunk) = src.get_next().await.unwrap() {
        chunks += 1;
    }
    assert_eq!(chunks, 2);

    // ...or everything at once.
    let src = Chunks::new(vec![b"hello ".to_vec(), b"world".to_vec()]);
    assert_eq!(src.get_size(), Some(11));
    assert_eq!(src.into_vec().await.unwrap(), b"hello world");
});
```

`into_vec()` makes the first chunk the result itself. Nothing is allocated or copied for a chunk nobody else holds a part of — one made of a `Vec<u8>`, for example. A chunk that shares its buffer, such as a part of what was read off a socket, is copied out of it.

- **The chunk is the whole stream** — its length is `get_size()`. It is returned as it is, and `get_next()` is not called again.
- **The size is known and the chunk is smaller.** The chunk is extended to that size at once, so it does not grow while the rest is appended.
- **The size is not known.** The rest is appended to the chunk until `get_next()` returns `None`.

The contracts:

- **Everything takes `&self`.** The position lives in an atomic or behind a lock, and the stream works as `Arc<dyn AsyncBytesStream<TError> + Send + Sync>`, `into_vec()` included. An `Arc` of a stream is a stream itself, so it goes wherever one is taken.
- **A chunk is handed over with no copy.** A stream that already has `Bytes` returns them as they are, and a `Vec<u8>` becomes a chunk with `.into()`.
- **`get_size()` must be exact.** `into_vec()` stops at a first chunk of exactly that length. It is asked after the first chunk has arrived, so a size that becomes known only then still counts.
- **`into_vec()` reads what is left.** Called after some `get_next()`, it returns only the remaining bytes.
- **Errors.** `into_vec()` returns the first error, and the chunks read before it are dropped.

## BufferedReader — parsing a stream cut into chunks

A stream is cut into chunks wherever it happens to be, so a JSON, a line or a frame may begin in one chunk and end in the next. `BufferedReader` wraps an `AsyncBytesStream` and keeps what is read and not parsed yet as one run of bytes. It needs no feature.

- `as_slice()` is what there is to parse.
- `mark_as_read(size)` drops `size` parsed bytes from the beginning.
- `get_next()` reads the next chunk in when there is not enough to parse. It returns all there is to parse — the bytes of `as_slice()` — as a `Bytes`.
- `read_mode()` is `false` once the stream is read to its end.

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use rust_extensions::bytes::Bytes;
use rust_extensions::{AsyncBytesStream, BufferedReader};

struct Chunks {
    chunks: Vec<Bytes>,
    next_chunk: AtomicUsize,
}

#[async_trait::async_trait]
impl AsyncBytesStream<std::io::Error> for Chunks {
    async fn get_next(&self) -> std::io::Result<Option<Bytes>> {
        let chunk_no = self.next_chunk.fetch_add(1, Ordering::Relaxed);
        Ok(self.chunks.get(chunk_no).cloned())
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
        chunks: vec![
            Bytes::from_static(b"{\"id\":1}\n{\"i"),
            Bytes::from_static(b"d\":2}\n{\"id\""),
            Bytes::from_static(b":3}\n"),
        ],
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

The bytes are kept in a `Bytes`, and they are copied only to be joined:

- **Nothing is left to parse.** The next chunk becomes the buffer as it is, with no copy.
- **Some bytes are not parsed yet.** The chunk is copied behind them. The buffer is reused: the room of what was marked as read is given back, and the buffer grows the way a `Vec` does.
- **`mark_as_read()` moves nothing.** The bytes that are left stay where they are.

The contracts:

- **What `get_next()` returns shares the buffer.** It is not a copy, and it stays as it is whatever is read later: keep it, slice it, send it to another task. While it is alive the reader can not put the next chunk into that buffer, so the next `get_next()` copies the bytes not parsed yet to a new one. For a piece that lasts for many chunks that is a copy of everything read so far on every call. Drop it before the next call, or parse through `as_slice()`, unless it is there to be kept.
- **`read_mode()` turns `false` one call after the last chunk.** The reader learns about the end when the stream returns `None`. That call returns what is left, and the stream is not asked again.
- **Chunks with no bytes are skipped.** After `get_next()` there is something new to parse, or `read_mode()` is `false`.
- **Errors.** `get_next()` returns the error of the stream as it is. What was read stays, and the next call asks the stream again.
- **`mark_as_read()` panics** when there are fewer than `size` bytes.
- **One parser.** Unlike the stream, the reader changes through `&mut self`.

## DoubleBuffer — two buffers between reading and parsing

One task reads a stream, another one parses it. `DoubleBuffer` gives them two buffers to pass the bytes through: while a chunk is being parsed, the next one is read into the other buffer. No bytes are copied, and there are never more than the two buffers. It needs no feature and no runtime.

`DoubleBuffer::new(buffer_size)` returns the two ends.

- **`DoubleBufferWriter`** is for the one who reads. `get_buffer_to_read()` gives a buffer — a `&mut [u8]` of `buffer_size` bytes — and waits until one of the two is free. `send(size)` of that buffer hands over the first `size` bytes of it.
- **`DoubleBufferReader`** is for the one who parses. `get_next()` gives what is read — a chunk, which is a `&[u8]` — and waits until there is one. Dropping the chunk frees its buffer to be read into again.

```rust
use rust_extensions::DoubleBuffer;
use tokio::io::AsyncReadExt;

let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();

rt.block_on(async {
    let (writer, reader) = DoubleBuffer::new(8);

    // The one who reads
    let reading = tokio::spawn(async move {
        let mut src: &[u8] = b"hello world, hello buffers"; // a socket, a file

        // None - the reader is dropped, so nobody is going to parse
        while let Some(mut buffer) = writer.get_buffer_to_read().await {
            let size = src.read(&mut buffer).await?;

            if size == 0 {
                break; // the stream is over
            }

            buffer.send(size);
        }

        std::io::Result::Ok(())
        // `writer` is dropped here - that is the end for the one who parses
    });

    // The one who parses
    let mut result = Vec::new();

    while let Some(chunk) = reader.get_next().await {
        assert!(chunk.len() <= 8);
        result.extend_from_slice(&chunk);
        // `chunk` is dropped here - its buffer is free to be read into again
    }

    assert_eq!(result, b"hello world, hello buffers");

    // `None` does not say why the reading is over - the task does
    reading.await.unwrap().unwrap();
});
```

A chunk becomes a `Bytes` with `into_bytes()`, which is what a chunk of an `AsyncBytesStream` is. It is not a copy: the buffer is free once the last piece of that `Bytes` — a clone of it, a slice of it — is dropped. So a stream over a `DoubleBufferReader` is `reader.get_next().await.map(DoubleBufferChunk::into_bytes)`, and a `BufferedReader` parses it as any other stream.

The contracts:

- **The one who reads is at most two buffers ahead.** With both buffers sent and not parsed yet, `get_buffer_to_read()` waits.
- **A chunk holds its buffer.** Parse it and drop it. Whoever keeps both chunks — or a piece of the `Bytes` of both — and calls `get_next()` waits forever, since there is nothing to read into. `BufferedReader` holds one chunk at most: the bytes not parsed yet are copied out of it when the next chunk comes.
- **A buffer dropped with no `send()` is free again.** That is what a read which was given up leaves — a `select!` which took the other branch, for example. `send(0)` does the same.
- **A buffer is not cleared.** What was read into it before is still there, so only the first `size` bytes of a read mean something.
- **The end goes both ways.** With the writer dropped, `get_next()` gives what was sent before and `None` after it. With the reader dropped, `get_buffer_to_read()` gives `None`, and what was sent and not taken is dropped.
- **`None` is not an error.** A reading task which has failed drops its writer just the same. It tells about the failure its own way — by what the task returns, as above.
- **`send()` panics** when the buffer is smaller than `size` bytes.
- **Everything takes `&self`**, and waiting needs no runtime — it works under `tokio` and in a browser alike.

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
