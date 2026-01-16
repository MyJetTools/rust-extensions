rust-extensions
==============

Utility crate of composable building blocks for time handling, strings, binary helpers, collection ergonomics, and small async/Tokio primitives.

## Install

```toml
[dependencies]
rust-extensions = { tag = "${last_tag}", git = "https://github.com/MyJetTools/rust-extensions.git" }
```

### Feature flags

- `with-tokio` — Tokio-backed helpers: timers, queues, events loop, task completions, application state tracking, sortable IDs.
- `base64` — Enable base64 encode/decode utilities.
- `hex` — Enable hex helpers.
- `objects-pool` — Object pooling.
- `vec-maybe-stack` — Stack-or-heap small-buffer optimization helpers.

Example:

```toml
rust-extensions = { version = "${last_tag}", features = ["with-tokio", "base64"] }
```

## Module map (what you get)

- Time: `date_time`, `duration_utils`, `stop_watch`, `atomic_stop_watch`, `atomic_duration`.
- String ergonomics: `short_string`, `maybe_short_string`, `string_builder`, `str_utils`, `str_or_string`, `as_str`.
- Binary helpers: `binary_payload_builder`, `binary_search`, `uint32_variable_size`, optional `base64`, optional `hex`.
- Collections & memory: `sorted_vec`, `sorted_ver_with_2_keys`, `grouped_data`, `auto_shrink`, `slice_or_vec`, `vec_maybe_stack` (opt), `objects_pool` (opt), `lazy`, `linq`, `array_of_bytes_iterator`, `slice_of_u8_utils`.
- Async/Tokio (feature `with-tokio`): `events_loop`, `my_timer`, `task_completion`, `tokio_queue`, `queue_to_save`, `application_states`, `sortable_id`.
- IO & misc: `file_utils`, `remote_endpoint`, `logger`, `min_value`, `max_value`, `min_key_value`, `placeholders`, `maybe_short_string`.

## Quick recipes

- Time point + interval keys:
  - `date_time::DateTimeAsMicroseconds` for UTC timestamps with µs precision.
  - `date_time::interval_key::*` for rounding/grouping into year/month/day/hour/minute/minute5 buckets.
- High-performance strings:
  - `ShortString` (Pascal-style, single-byte length, max 255 bytes on stack) with `Display`, `Serialize`, `Eq`, hashing.
- `MaybeShortString` keeps data inline as `ShortString` when length ≤ 255 bytes; seamlessly upgrades to `String` when longer.
  - `StringBuilder` for incremental push/format operations.
- Binary payloads:
  - `BinaryPayloadBuilder` to append scalars, slices, and length-prefixed data into a single `Vec<u8>`.
  - `Uint32VariableSize` for compact integer encoding/decoding.
- Collections:
  - `SortedVec` family: `SortedVec`, `SortedVecWithStrKey`, `SortedVecOfArc`, `SortedVecOfArcWithStrKey`, and `SortedVecWith2Keys` maintain order on insert and support efficient lookups.
  - `AutoShrinkVec` / `AutoShrinkVecDeque` resize toward steady-state usage.
  - `SliceOrVec` toggles between borrowed and owned buffers.
- Async/Tokio (enable `with-tokio`):
  - `MyTimer` for tick-driven tasks, `EventsLoop` for fan-out processing, `TaskCompletion` for awaiting completion handles, `TokioQueue` for bounded async queues, `QueueToSave` for producer/consumer disk pipelines, `ApplicationStates` for async state transitions.
- File/IO:
  - `file_utils::read_file_lines_iter`, `array_of_bytes_iterator::FileIterator`, `remote_endpoint` helpers for host/port parsing.

## Time utilities in detail

`DateTimeAsMicroseconds` is a single-field UTC timestamp (`unix_microseconds: i64`) with serde support and helpers to add/subtract durations, compare, format (RFC 3339/2822/5322/7231, compact), and convert to `chrono::DateTime<Utc>`.

Interval keys let you cut timestamps to buckets:
- Compile-time typed: `IntervalKey<YearKey | MonthKey | DayKey | HourKey | MinuteKey | Minute5Key>`.
- Runtime enum: `DateTimeInterval::{Year, Month, Day, Hour, Minute, Min5}`.
- Conversions are zero-cost wrappers over `i64` values; you can go from `DateTimeAsMicroseconds` to an interval key and back.

Minimal example:

```rust
use rust_extensions::date_time::*;
use rust_extensions::date_time::interval_key::*;
use std::time::Duration;

let now = DateTimeAsMicroseconds::now();
let minute_key: IntervalKey<MinuteKey> = now.into();
let next_minute = minute_key.add(Duration::from_secs(60));
assert!(next_minute.to_i64() >= minute_key.to_i64());
```

## Strings in detail

- `ShortString`: Pascal-style layout (length stored in the first byte) backed by `[u8; 256]`, so the maximum encoded length is 255 bytes. Supports UTF-8 chars, checked `try_push`/`try_push_str` and panicking `push`/`push_str`, serde, comparison, and case-insensitive helpers. Ideal for small IDs, headers, and keys without heap allocations.
- `MaybeShortString`: stores as `ShortString` while length ≤ 255 bytes; automatically promotes to `String` once it would overflow, so you can push without manual branching.
- `StrOrString` / `SliceOrVec` for zero-copy borrow-or-own patterns.
- `StringBuilder`: push bytes/strings/char, drain to `String` without realloc churn.

Example:

```rust
use rust_extensions::ShortString;

let mut s = ShortString::from_str("hi").unwrap();
assert!(s.try_push('-'));
s.push_str("there");
assert_eq!(s.as_str(), "hi-there");
```

## Binary helpers

- `BinaryPayloadBuilder`: append primitives (`u8`, `u16`, `u32`, `u64`, slices) and length-prefixed blobs; get the final `Vec<u8>` or borrowed slice.
- `Uint32VariableSize`: encode variable-length `u32` values for compact wire/storage formats.
- Optional: `base64` and `hex` modules expose encode/decode helpers compatible with the rest of the crate.

Example:

```rust
use rust_extensions::binary_payload_builder::BinaryPayloadBuilder;

let mut builder = BinaryPayloadBuilder::new();
builder.write_u16(42);
builder.write_slice(b"ping");
let bytes = builder.finish();
assert_eq!(bytes.len(), 2 + 4);
```

## Collections & memory helpers

- `SortedVec<T>` / `SortedVecWith2Keys<K1, K2, V>` keep elements ordered; provide binary search insertion and lookup APIs.
- `GroupedData` to collect items by key with minimal allocations.
- `AutoShrinkVec` / `AutoShrinkVecDeque` shrink capacity after spikes.
- `ObjectsPool` (feature `objects-pool`) for pooling reusable buffers/objects.
- `VecMaybeStack` (feature `vec-maybe-stack`) for small-buffer-optimized vectors.
- Iteration helpers: `array_of_bytes_iterator::{SliceIterator, VecIterator, FileIterator}`, `slice_of_u8_utils` for safe chunking.
- `Lazy<T>` for deferred construction guarded by `OnceLock`-like behavior.

## Async & Tokio (feature `with-tokio`)

- `EventsLoop`: process tasks with configurable parallelism.
- `MyTimer`: tick-based scheduling with graceful stop.
- `TaskCompletion`: create awaitable completion sources with error support.
- `TokioQueue`: bounded async queue with backpressure.
- `QueueToSave`: producer/consumer file-saving pipeline with retries.
- `ApplicationStates`: async state machine with callbacks.
- `SortableId`: monotonic sortable IDs backed by time + randomness.

```rust
#[cfg(feature = "with-tokio")]
async fn example_queue() {
    use rust_extensions::tokio_queue::TokioQueue;

    let queue = TokioQueue::new(100);
    queue.send("item").await.unwrap();
    let item = queue.recv().await.unwrap();
    assert_eq!(item, "item");
}
```

## IO, logging, misc

- `file_utils`: iterators over file lines and path helpers.
- `logger`: simple structured logger traits.
- `remote_endpoint`: parse/format endpoints (`host:port`), detect loopback.
- Math/min-max helpers: `min_value`, `max_value`, `min_key_value`, `max_value`.
- `StopWatch` / `AtomicStopWatch` / `AtomicDuration` for timing.

## Development status

- License: MIT (see `LICENSE.md`).
- Current crate version: **0.1.5**.
- Contributions are welcome via pull requests.

