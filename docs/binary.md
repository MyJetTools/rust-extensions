# binary

Build and read byte payloads: integers, variable-size lengths, borrowed-or-owned buffers, byte search, cursors over bytes, byte streams read in chunks, hex and base64.

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

## AsyncBytesReader — bytes read in chunks

A trait for a source of bytes that arrives chunk by chunk: a file, a response body, a blob downloaded in parts. It needs no feature.

- `get_next()` returns `Ok(Some(chunk))` with the next chunk, and `Ok(None)` at the end of the data.
- `get_size()` returns the size of the whole stream in bytes, or `None` when it is not known before the stream is read.
- `into_vec()` is already implemented. It reads the stream to the end and returns everything as one `Vec<u8>`. A known size is allocated at once, so the `Vec` does not grow while the chunks are appended.

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use rust_extensions::AsyncBytesReader;

struct Chunks {
    chunks: Vec<Vec<u8>>,
    next_chunk: AtomicUsize,
}

impl Chunks {
    fn new(chunks: Vec<Vec<u8>>) -> Self {
        Self { chunks, next_chunk: AtomicUsize::new(0) }
    }
}

#[async_trait::async_trait]
impl AsyncBytesReader<std::io::Error> for Chunks {
    async fn get_next(&self) -> std::io::Result<Option<Vec<u8>>> {
        let chunk_no = self.next_chunk.fetch_add(1, Ordering::Relaxed);
        Ok(self.chunks.get(chunk_no).cloned()) // None once the chunks are over
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

- **Everything takes `&self`.** The position lives in an atomic or behind a lock, and the source works as `Arc<dyn AsyncBytesReader<TError> + Send + Sync>`.
- **`into_vec()` reads what is left.** Called after some `get_next()`, it returns only the remaining bytes.
- **Errors.** `into_vec()` returns the first error, and the chunks read before it are dropped.

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
