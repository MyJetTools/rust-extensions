rust-extensions
==============

Utility crate of composable building blocks for time handling, strings, binary helpers, collection ergonomics, and small async/Tokio primitives.

## Install

```toml
[dependencies]
rust-extensions = { tag = "${last_tag}", git = "https://github.com/MyJetTools/rust-extensions.git" }
```

### Feature flags

- `with-tokio` — Tokio-backed helpers: timers, queues, events loop, task completions, application state tracking. Turns `rnd` on as well.
- `rnd` — Everything which needs a source of randomness: `uuid::generate_v4()` and `SortableId` (sortable IDs).
- `base64` — Enable base64 encode/decode utilities.
- `hex` — Enable hex helpers.
- `objects-pool` — Object pooling.
- `vec-maybe-stack` — Stack-or-heap small-buffer optimization helpers.

Example:

```toml
rust-extensions = { tag = "${last_tag}", git = "https://github.com/MyJetTools/rust-extensions.git", features = ["with-tokio", "base64"] }
```

## Module map (what you get)

- Time: `date_time`, `duration_utils`, `stop_watch`, `atomic_stop_watch`, `atomic_duration`.
- String ergonomics: `short_string`, `maybe_short_string`, `string_builder`, `secure_string_builder`, `str_utils`, `str_or_string`, `as_str`.
- Binary helpers: `binary_payload_builder`, `binary_search`, `uint32_variable_size`, optional `base64`, optional `hex`.
- Collections & memory: `sorted_vec`, `sorted_ver_with_2_keys`, `grouped_data`, `auto_shrink`, `slice_or_vec`, `sized_chunks`, `vec_maybe_stack` (opt), `objects_pool` (opt), `lazy`, `linq`, `array_of_bytes_iterator`, `slice_of_u8_utils`, `vec_uninit!`.
- Async/Tokio (feature `with-tokio`): `events_loop`, `background_executor`, `background_executor_with_multi_threads`, `my_timer`, `exact_timer`, `task_completion`, `is_initialized`, `idempotency`, `tokio_queue`, `queue_to_save`, `queue_to_save_with_id`, `queue_to_save_or_delete_with_id`, `startable`, `application_states`.
- Randomness (feature `rnd`, on with `with-tokio`): `uuid`, `sortable_id`.
- IO & misc: `file_utils`, `remote_endpoint`, `logger`, `min_value`, `max_value`, `min_key_value`, `maybe_short_string`.

## Quick recipes

- Time point + interval keys:
  - `date_time::DateTimeAsMicroseconds` for UTC timestamps with µs precision.
  - `date_time::DateTimeAsMicrosecondsWithTimeZone` (+ `TimeZone`) to pair a UTC instant with an offset and render it as local wall-clock time.
  - `date_time::IntervalKey` / `date_time::DateTimeInterval` for rounding/grouping into year/month/week/day/hour (1h/2h/4h)/minute (1m/5m/15m/30m) buckets.
- High-performance strings:
  - `ShortString` (Pascal-style, single-byte length, max 255 bytes on stack) with `Display`, `Serialize`, `Eq`, hashing.
- `MaybeShortString` keeps data inline as `ShortString` when length ≤ 255 bytes; seamlessly upgrades to `String` when longer.
  - `StringBuilder` for incremental push/format operations.
  - `SecureStringBuilder` for the same, when the content is a secret: the buffer never re-allocates itself — every retired allocation, and the final one on `Drop`, is overwritten with zeroes.
- Binary payloads:
  - `BinaryPayloadBuilder` to append integers (`u8` … `u64`, `i8` … `i64`), little-endian, into a `Vec<u8>` or into a `&mut [u8]` given by the caller.
  - `UInt32VariableSize` for compact integer encoding/decoding.
- Collections:
  - `SortedVec` family: `SortedVec`, `SortedVecWithStrKey`, `SortedVecOfArc`, `SortedVecOfArcWithStrKey`, `SortedVecWith2StrKey` and `SortedVecOfArcWith2StrKey` maintain order on insert and support efficient lookups.
  - `VecAutoShrink` / `VecDequeAutoShrink` resize toward steady-state usage.
  - `SliceOrVec` toggles between borrowed and owned buffers.
- Async/Tokio (enable `with-tokio`):
  - `MyTimer` for tick-driven tasks, `MyExactTimer` for ticks aligned to wall-clock marks, `EventsLoop` for fan-out processing, `BackgroundExecutor` to offload bursty work onto a single background task, `BackgroundExecutorWithMultiThreads` to do the same per `thread_id` — sequentially within one id, in parallel across ids, `TaskCompletion` for awaiting completion handles, `IsInitialized` as a one-shot initialization gate many tasks can await, `idempotency::by_process_id::IdempotencyCache` to make a retried request execute at most once (`idempotency::by_user_id_and_process_id::IdempotencyCache` when a process id is only unique within its user), `TokioQueue` for an in-memory byte pipe read through `AsyncRead`, `QueueToSave` / `QueueToSaveAsBulk` for producer/consumer save pipelines, `ApplicationStates` / `AppStates` for the initialized / shutting-down flags of the application.
- File/IO:
  - `file_utils::format_path` / `file_utils::FilePath`, `array_of_bytes_iterator::FileIterator`, `remote_endpoint` helpers for host/port parsing.

## Time utilities in detail

`DateTimeAsMicroseconds` is a single-field UTC timestamp (`unix_microseconds: i64`) with serde support (see [Serde format](#datetimeasmicroseconds-serde-format)) and helpers to add/subtract durations, compare, format (RFC 3339/2822/5322/7231, compact), and convert to `chrono::DateTime<Utc>`.

Constructors include `new(unix_microseconds)`, `now()`, `create(...)`, `from_str`, `parse_iso_string`, and `from_nanos(value: i64)` — which converts a Unix nanoseconds timestamp to µs (valid range ~1677–2262).

`From<i64>` (`let dt: DateTimeAsMicroseconds = value.into()`) auto-detects the unit of a Unix timestamp by magnitude — seconds, milliseconds, microseconds, or nanoseconds — and normalizes it to microseconds.

### `DateTimeAsMicroseconds` serde format

**The impls are hand-written and deliberately asymmetric. Do not "tidy" them into a symmetric pair, and do not restore `#[serde(transparent)]`.**

- **Serialize → always RFC 3339**, UTC, `Z` suffix, fixed microsecond precision: `"2021-04-25T17:30:03.000000Z"`. Never a number. This is what OpenAPI/JSON Schema `format: date-time` promises, and what protobuf JSON, the Google API Design Guide, and my-http-utils' schema + client writer all already speak.
- **Deserialize → tolerant, accepts both**:

| JSON input | Read as |
| --- | --- |
| `"2021-04-25T17:30:03.000000Z"` | RFC 3339 (the format we now write) |
| `"2021-04-25T17:30:03+00:00"` | RFC 3339 with a numeric zero offset — what the older `to_rfc3339()` emits |
| `"1619371803000000"` | digits in a string → unix timestamp, unit sniffed by magnitude via `From<i64>` |
| `1619371803000000` | number → unix timestamp, unit sniffed by magnitude via `From<i64>` |

Why asymmetric: changing the *write* format is not an API break the compiler can catch — it is a break of **data already at rest**. Across the monorepo `DateTimeAsMicroseconds` sits in MyNoSql entities, settings files, Service Bus messages, jsonb columns and caches, where history is written as `1704164645000000`. A strict reader would make those unreadable, and rolling the deploy back would not heal them. Writing the new format while reading both (a *tolerant reader*) is the standard format-migration move.

A **number carries no unit**, so — quoted or bare — it always goes through `From<i64>`, which sniffs seconds / millis / micros / nanos by magnitude. One rule for numbers everywhere in the crate; `deserialize_number_sniffs_the_unit` pins it. Data written by the old `#[serde(transparent)]` impl reads back unchanged, because a real microseconds timestamp (`1704164645000000` ≈ 1.7e15) sits in the microseconds band. The sniffing only reinterprets a bare number below ~4.7e9 — the first ~78 minutes after the epoch, where `0` maps to `0` regardless — so no realistic stored timestamp moves.

Deserialization uses `deserialize_any`, so it needs a **self-describing** format (JSON — what this monorepo uses). It would not work under bincode/postcard/rmp.

#### `from_json_value_str` — the entry point for non-serde deserializers

`DateTimeAsMicroseconds::from_json_value_str(src: &str) -> Option<Self>` takes a **raw JSON value token**, exactly as a parser hands it over — a quoted string keeps its quotes, a number arrives bare (this is what my-json's `JsonValueRef::as_raw_str()` returns). Use it from any hand-written deserializer (my-json, my-http-utils) so every reader accepts the same spellings:

```rust
DateTimeAsMicroseconds::from_json_value_str("\"2021-04-25T17:30:03.000000Z\"");  // RFC 3339, Z
DateTimeAsMicroseconds::from_json_value_str("\"2021-04-25T17:30:03+00:00\"");    // older to_rfc3339()
DateTimeAsMicroseconds::from_json_value_str("\"1619371803\"");                   // quoted number
DateTimeAsMicroseconds::from_json_value_str("1619371803000000");                 // bare number
DateTimeAsMicroseconds::from_json_value_str("null");                             // -> None
```

The rule is simply: **quotes present → strip them; then parse the content**. Quotes are packaging, not meaning — `"1619371803"` and `1619371803` land on the same instant, because either way the digits go to `From<i64>` for unit sniffing. Single quotes are accepted alongside double ones (matching my-json), escapes are not resolved (no RFC 3339 spelling contains one), and `null` / empty / garbage give `None` rather than a panic. If you have already stripped the quotes, `from_str` is equivalent.

serde does **not** call `from_json_value_str`, and cannot: a `Deserializer` hands the visitor an already-decoded value, having consumed the quotes. Instead both routes bottom out in the same primitives — `from_str` for a string, `From<i64>` for a number — and the test `serde_and_from_json_value_str_agree` pins them together across every spelling, so the two cannot drift.

`Display`, `Debug` and serde are three different renderings — don't reach for the wrong one:

```rust
let dt = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:03.000Z").unwrap();

dt.to_string();                     // Display  -> "1619371803000000"  (a number!)
format!("{:?}", dt);                // Debug    -> "'2021-04-25T17:30:03+00:00'"
serde_json::to_string(&dt).unwrap();// serde    -> "\"2021-04-25T17:30:03.000000Z\""
```

`to_rfc3339()` (chrono's default, renders the zero offset as `+00:00`) and `to_rfc3339_utc()` (`Z` suffix, fixed 6-digit fraction, the serde wire format) are both available; both parse back. Prefer `to_rfc3339_utc()` when the string gets stored or sorted — its fixed width makes lexicographic order match chronological order.

Note the parsers ignore a **non-zero** timezone offset: `2024-01-02T03:04:05+03:00` reads as `03:04:05` UTC, not `00:04:05`. `Z` and `+00:00` are unaffected.

Interval keys let you cut timestamps to buckets:
- Compile-time typed: `IntervalKey<YearKey | MonthKey | WeekMondayKey | WeekSundayKey | DayKey | HourKey | Hour2Key | Hour4Key | MinuteKey | Minute5Key | Minute15Key | Minute30Key>`.
- Runtime enum: `DateTimeInterval::{Year, Month, WeekMonday, WeekSunday, Day, Hour, Hour2, Hour4, Minute, Min5, Min15, Min30}`.
- Each key is encoded as an `i64` whose numeric order matches chronological order **within a given key type**: calendar fields packed as digits (e.g. `YYYYMMDDHHmm`), with sub-hour/sub-day keys normalized to the slot start. Week keys encode the `YYYYMMDD` date of the week start (Monday- or Sunday-based), so a week key shares the `DayKey` layout — values of different key types are not mutually comparable.
- Conversions are zero-cost wrappers over `i64` values; you can go from `DateTimeAsMicroseconds` to an interval key and back. `from_i64` is unchecked, so pass only a value previously produced for the same key type.

Minimal example:

```rust
use rust_extensions::date_time::*;
use std::time::Duration;

let now = DateTimeAsMicroseconds::now();
let minute_key: IntervalKey<MinuteKey> = now.into();
let next_minute = minute_key.add(Duration::from_secs(60));
assert!(next_minute.to_i64() >= minute_key.to_i64());
```

### `DateTimeAsMicrosecondsWithTimeZone` — a UTC instant with a timezone

`DateTimeAsMicroseconds` is always UTC. When you need to render a moment back into the **local wall-clock time** it was captured in, pair it with a `TimeZone`:

```rust
pub struct DateTimeAsMicrosecondsWithTimeZone {
    pub date_time: DateTimeAsMicroseconds, // the instant, kept in UTC
    pub time_zone: TimeZone,               // the offset to render it in
}
```

`TimeZone` is a fixed offset from UTC stored **in minutes** — `UTC+1` = `60`, `UTC-5` = `-300`, `UTC+5:45` (Nepal) = `345`:

- `TimeZone::utc()` / `TimeZone::from_minutes(i32)` — construct directly.
- `TimeZone::from_server_and_local_time(server_utc, local)` — derive the offset as `local - server`, **rounded to the nearest 15 minutes** (15-minute steps exist in the wild, so `+58m` → `+60`, `+50m` → `+45`, `+7m` → `0`). `local` is the same moment as read on the local clock, encoded as a plain `DateTimeAsMicroseconds`.
- `offset_in_minutes()` / `offset_in_seconds()` / `to_fixed_offset()` — read it back; `Debug` renders as `+01:00`.

Human-visible renderings (all apply the offset first):

- `to_local_date_time_struct() -> DateTimeStruct` — local `year`/`month`/`day`/`time`/`dow`, correct across midnight and for negative offsets.
- `to_rfc3339()` — `2021-04-25T18:30:03.000000+01:00` (numeric offset, fixed µs precision). This is also the **serde wire format**: it serializes to that one string and reads it back (a trailing `Z` is `UTC+0`); a bare number or an offset-less string is rejected rather than silently misread.
- `to_compact_string()` — `2021-04-25 18:30:03` (local time, no zone suffix).

```rust
use rust_extensions::date_time::*;

let server = DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:03.000Z").unwrap();

// Client reported its wall clock as 18:30 for the same moment -> derive UTC+1.
let mut local = server;
local.add_minutes(58); // a noisy +58m rounds to +60
let dt = DateTimeAsMicrosecondsWithTimeZone::from_server_and_local_time(server, local);

assert_eq!(60, dt.time_zone.offset_in_minutes());
assert_eq!("2021-04-25 18:30:03", dt.to_compact_string());
assert_eq!("2021-04-25T18:30:03.000000+01:00", dt.to_rfc3339());
```

## Strings in detail

- `ShortString`: Pascal-style layout (length stored in the first byte) backed by `[u8; 256]`, so the maximum encoded length is 255 bytes. Supports UTF-8 chars, checked `try_push`/`try_push_str` and panicking `push`/`push_str`, serde, comparison, and case-insensitive helpers. Ideal for small IDs, headers, and keys without heap allocations.
- `MaybeShortString`: stores as `ShortString` while length ≤ 255 bytes; automatically promotes to `String` once it would overflow, so you can push without manual branching.
- `StrOrString` / `SliceOrVec` for zero-copy borrow-or-own patterns.
- `StringBuilder`: push bytes/strings/char, drain to `String` without realloc churn.
- `SecureStringBuilder`: a `StringBuilder` for passwords, tokens, connection strings and private keys — see below.

Example:

```rust
use rust_extensions::ShortString;

let mut s = ShortString::from_str("hi").unwrap();
assert!(s.try_push('-'));
s.push_str("there");
assert_eq!(s.as_str(), "hi-there");
```

### `SecureStringBuilder` — a builder that does not leave the secret behind

A plain `String` grows by handing the old block back to the allocator **as-is**: `realloc` copies the bytes into the new block and frees the old one untouched, so a secret that grew from 8 to 16 to 32 bytes leaves three readable copies of itself in the heap's free lists — and dropping the string frees the last one just as untouched. Whatever reads that memory next (the next allocation, a core dump, a swapped-out page, a heap-scanning exploit) reads the secret.

`SecureStringBuilder` never lets its inner `String` re-allocate. It watches every push, and the moment the next one would not fit it allocates a new buffer, copies the content over, **overwrites the whole old allocation with zeroes** — every byte of its `capacity()`, not only the `len()` that was in use — and only then frees it. `Drop` and `clear()` do the same to the current buffer.

```rust
use rust_extensions::SecureStringBuilder;

let mut builder = SecureStringBuilder::new();

builder.push_str("postgres://user:");
builder.push_str("s3cr3t");
builder.push('@');
builder.push_str("localhost:5432");

assert_eq!("postgres://user:s3cr3t@localhost:5432", builder.as_str());

// connect(builder.as_str()) ...
// Dropping here zeroes the whole allocation before freeing it.
```

- **The wipe cannot be optimised away** — it is a byte-by-byte `ptr::write_volatile` followed by a `compiler_fence`. A plain `memset` right before a free is dead code the optimiser is entitled to delete, and a `.fill(0)` that got deleted looks exactly like one that works.
- **The whole capacity is zeroed, not just the length** — the bytes past `len` hold whatever a longer earlier content left there, and they are freed along with the rest.
- **Nothing owned ever comes out** — the content is handed over by reference only, through `as_str()` and `as_slice()`. There is deliberately no `into_string()`, no `Clone` and no `Display` (which would hand out `.to_string()`): each of them would put a copy of the secret into a `String` nobody wipes. `Debug` prints the shape (`len`, `capacity`) and never the content.
- **Pre-size when the length is known** — `with_capacity(n)` means the secret is only ever written to one address, so there is a single allocation to wipe instead of a chain of them. `reserve(n)` does the same mid-flight.
- **`clear()` keeps the capacity** — it zeroes the allocation and empties the builder, so re-filling it does not allocate again.

API: `new` / `with_capacity`, `push_str` / `push(char)` / `push_line` / `push_bytes` (UTF-8 checked), `as_str` / `as_slice`, `len` / `capacity` / `is_empty`, `reserve`, `clear`.

Not a defence against an attacker who can read the process while the value is alive, and it does not lock pages into RAM — it stops the secret from **outliving its use** in memory the process no longer owns. Keep the builder short-lived.

## Binary helpers

- `BinaryPayloadBuilder`: append integers — `write_u8` … `write_u64`, `write_i8` … `write_i64` — either to a growing `Vec<u8>` (`new_as_vec()`) or into a buffer of the caller (`new_as_slice(&mut [u8])`). Multi-byte integers are written **little-endian**, and both modes give the very same bytes for the same writes. `.into()` turns the builder into a `SliceOrVec<u8>` with what was written: the whole `Vec`, or — in the slice mode — the written part of the buffer, not the whole of it. A write which does not fit into the buffer of the caller panics.
- `UInt32VariableSize`: encode variable-length `u32` values for compact wire/storage formats.
- Optional: `base64` and `hex` modules expose encode/decode helpers compatible with the rest of the crate.

Example:

```rust
use rust_extensions::{BinaryPayloadBuilder, SliceOrVec};

let mut builder = BinaryPayloadBuilder::new_as_vec();
builder.write_u16(0x0102);
builder.write_u32(7);
let bytes: SliceOrVec<u8> = builder.into();
// Little-endian: the lowest byte goes first.
assert_eq!(bytes.as_slice(), &[0x02, 0x01, 7, 0, 0, 0]);
```

## Collections & memory helpers

- `SortedVec<TKey, TValue>` / `SortedVecWith2StrKey<TValue>` keep elements ordered; provide binary search insertion and lookup APIs. The key is taken from the element itself: `EntityWithKey<TKey>` for the former, `EntityWith2StrKey` — a primary and a secondary `&str` key — for the latter.
- `grouped_data`: `group_to_hash_map` / `group_to_btree_map` collect an iterator into `Vec`s by key; `GroupedDataAsHashmap` / `GroupedDataAsBTreeMap` keep values as group key → key → value and drop a group once its last value is removed.
- `VecAutoShrink` / `VecDequeAutoShrink` shrink capacity after spikes.
- `ObjectsPool` (feature `objects-pool`) for pooling reusable buffers/objects.
- `VecMaybeStack` (feature `vec-maybe-stack`) for small-buffer-optimized vectors.
- Iteration helpers: `array_of_bytes_iterator::{SliceIterator, VecIterator, FileIterator}`, `slice_of_u8_utils::SliceOfU8Ext` to find a byte, a byte sequence or the first byte matching a condition in a `[u8]`.
- `lazy::{LazyVec, LazyHashMap, LazyGroupIntoHashMap, LazyGroupIntoBTreeMap}` collect into a container which is allocated on the first insert only — `get_result()` gives `None` when nothing was added.
- `split_into_sized_chunks` / `SizeBudget` cut a collection into batches by MEASURED size instead of item count — see below.
- `vec_uninit![len]` allocates a `Vec<u8>` of `len` bytes without zeroing them — a read buffer for a socket. The bytes are garbage until written: look only at the part a read reported (`&buf[..n]`). Allows `clippy::uninit_vec` inside its expansion, so the call site needs no `#[allow]`.

```rust
use rust_extensions::vec_uninit;

let mut buf = vec_uninit![1024 * 1024];
// let n = socket.read(&mut buf).await?;
// process(&buf[..n]);
```

### Batching by size, not by count

Every real batching limit is expressed in **bytes**: a gRPC message, a request body, a datagram, a row batch handed to a driver. Cutting such a batch every N items is therefore a guess about the average item, and the guess fails exactly when items are unusually large — the case nobody has test data for, and the one that shows up in production as a batch that can never be delivered.

`split_into_sized_chunks` takes the cost function from the caller, so the crate stays free of any serialisation dependency and **the same helper serves a client and a server**: pass `prost::Message::encoded_len`, `serde_json::to_vec(..).len()`, a `str`'s `len`, or your own estimate — it is all just `usize`.

```rust
use rust_extensions::split_into_sized_chunks;

// One protobuf message per chunk, each safely under a 4 MiB decode limit.
for page in split_into_sized_chunks(rows, 3 * 1024 * 1024, |row| row.encoded_len() + 8) {
    producer.send(Response { page }).await?;
}
```

Two behaviours worth knowing, both deliberate:

- **An item bigger than the whole limit becomes a chunk of one** rather than being refused forever. Flushing cannot make it fit, so the choice is between passing it on — letting the real boundary reject it with its own error — and spinning. It is passed on.
- **An empty input yields no chunks at all**, so a caller that sends one message per chunk sends nothing rather than an empty message.

When a single batch must hold items of **several types** — two `repeated` fields of one protobuf message, say — drive `SizeBudget` directly instead, so the types share a batch rather than getting one each:

```rust
use rust_extensions::SizeBudget;

let mut budget = SizeBudget::new(3 * 1024 * 1024);

for deal in deals {
    let cost = deal.encoded_len() + 8;
    if budget.needs_flush(cost) {
        send(std::mem::take(&mut page)).await?;
        budget.reset();
    }
    budget.add(cost);
    page.deals.push(deal);
}
// ... same loop for `orders`, filling the same page ...
```

`needs_flush` always answers `false` on an empty batch, which is what makes the over-sized-item rule above fall out rather than needing a special case at every call site.

## Async & Tokio (feature `with-tokio`)

- `EventsLoop`: single-consumer async message loop — `send` is lock-free, the consumer runs in a dedicated Tokio task; `tick()` gets the event **by ownership** (no clone) and returns `RepeatIteration<TModel>`, so an unfinished iteration hands the very same model back via `Yes(model)` and is started again with it.
- `BackgroundExecutor`: offloads work from the caller onto a single background Tokio task — `trigger()` is lock-free, callable from **any** thread (including one with no Tokio runtime around it), and runs the registered `execute()` exactly once per call, never in parallel; `execute()` can return `RepeatIteration::Yes` to ask for another iteration.
- `BackgroundExecutorWithMultiThreads<TThreadId>`: the same, but split into independent threads by the `thread_id` given to `trigger()` — one thread id is served by one background task (sequentially, and the id is passed to `execute()`), different thread ids are served in parallel, and the task of a thread id is spawned on its first trigger and removed once its triggers are drained. `trigger()` is likewise callable from any thread.
- `MyTimer`: tick-based scheduling; `tick()` returns `RepeatTimerIteration` and can ask to be run again immediately. `start()` is called once — a second call panics.
- `MyExactTimer`: same tick model as `MyTimer` — and the same one-shot `start()` — but fires exactly on aligned wall-clock marks (`:00, :05, :10 …`) with no drift.
- `TaskCompletion`: create awaitable completion sources with error support.
- `IsInitialized`: one-shot initialization gate — any number of tasks `await` until initialization happens, then every subsequent wait flies through a lock-free atomic flag.
- `idempotency::by_process_id::IdempotencyCache` / `idempotency::by_user_id_and_process_id::IdempotencyCache`: de-duplicates retries of the same request — identified by a process id, or by a user id + process id pair — the first caller executes, concurrent retries park on the same execution, later retries get the memorized result.
- `TokioQueue`: an in-memory byte pipe — producers `enqueue(&[u8])` through `get_publisher()`, the queue itself is the reading side and implements `tokio::io::AsyncRead`. It is not bounded: nothing pushes back on the producers.
- `QueueToSave`: producer/consumer pipeline which hands the queued items to the handler one by one, by value — `execute(item)`. There are no retries: a panic or a timeout (10s) of the handler is logged and that item is lost.
- `QueueToSaveAsBulk`: the same queue flushed in chunks of up to 50 items — the handler receives `execute(&[T], attempt_no)`. Returning means saved; on a panic or a timeout (10s) the amount is logged and the same slice is handed over again after `retry_timeout` (1s, `set_retry_timeout`) with `attempt_no` + 1.
- `QueueToSaveWithId`: same producer/consumer batching as `QueueToSaveAsBulk`, but each item implements `PersistObjectId<ID>`. Re-enqueuing an item with an ID already in the queue overwrites the pending entry, so only the latest state per ID is flushed to the handler. `ID` must be `Hash + Eq + Clone + Debug`; the handler receives `execute(&[T], attempt_no)` per tick. Returning means saved; a panic prints the IDs of the chunk to the console, and on a panic or a timeout the amount is logged and the same slice is handed over again after `retry_timeout` (1s, `set_retry_timeout`) with `attempt_no` + 1. No ordering guarantee across IDs.
- `QueueToSaveOrDeleteWithId`: `QueueToSaveWithId` with two pending states per ID — upsert or delete. `enqueue_delete(id)` drops the pending object right there (there is nothing to save about an object which is about to be deleted) and leaves only the ID marked for deletion; a later `enqueue_single` of the same ID overwrites the delete back into an upsert. The handler receives `&[UpsertOrDelete<ID, T>]` with the same retry contract — `UpsertOrDelete::split(items)` cuts it into `(Vec<&T>, Vec<&ID>)` for a bulk insert-or-replace plus a bulk delete.
- `Startable`: `fn start(&self)` — implemented by `MyTimer`, `MyExactTimer`, `EventsLoop`, `BackgroundExecutor`, `BackgroundExecutorWithMultiThreads`, `QueueToSave`, `QueueToSaveAsBulk`, `QueueToSaveWithId` and `QueueToSaveOrDeleteWithId` (not available on wasm). Each of them gets everything `start` needs in `new` — the `logger`, plus `app_states` for `EventsLoop` — so they can be collected as `Vec<Arc<dyn Startable + Send + Sync + 'static>>` while the app is being wired up and started in one loop. The same `start()` is also an inherent method, so starting a single one needs no import. `EventsLoop` is the only one of them which waits for its `app_states` to be initialized; the timers, the executors and the queues are at work from `start()` on, so start them once the application is ready.
- `ApplicationStates`: the trait a component reads the application lifecycle through — `is_initialized()` and `is_shutting_down()`. `AppStates` is the ready implementation: two atomic flags (`create_un_initialized()` / `create_initialized()`, `set_initialized()`, `set_shutting_down()`) plus `wait_until_shutdown().await`, which hooks SIGTERM / SIGINT to raise the shutdown flag and returns once it is raised (`AppStates` is not available on wasm). Of the components above only `EventsLoop` reads it.
- `SortableId`: monotonic sortable IDs backed by time + randomness.

```rust
#[cfg(feature = "with-tokio")]
async fn example_queue() {
    use rust_extensions::tokio_queue::TokioQueue;
    use tokio::io::AsyncReadExt;

    let mut queue = TokioQueue::new();
    let publisher = queue.get_publisher();

    // Producers hold the publisher - `enqueue` takes `&self` and never awaits.
    publisher.enqueue(b"item");

    // The queue itself is the reading side - whatever takes an `AsyncRead` can drain it.
    let mut buf = [0u8; 4];
    queue.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"item");
}
```

### `EventsLoop` use case

`EventsLoop` is designed to live inside an `AppCtx` as a plain field (no outer `Mutex` / no `mut` access needed). The flow:

1. **Construct in `AppCtx::new`** — the channel is created immediately, so `send` is available right away and is lock-free (no mutex on the hot path).
2. **Register a callback** (`EventsLoopTick`) via `register_event_loop` — typically during app initialization, once dependencies are wired.
3. **Start** — spawns the background reader task which owns the receiver + callback and drives `started` / `tick` / `finished`. The reader holds until the `app_states` given to `new` is initialized.
4. **Send / stop** — `send(msg)` pushes a message; `stop()` sends a shutdown signal.

```rust
#[cfg(feature = "with-tokio")]
mod example {
    use std::sync::Arc;
    use rust_extensions::{
        events_loop::{EventsLoop, EventsLoopTick, RepeatIteration},
        ApplicationStates, Logger,
    };

    // 1. Define what each tick should do.
    struct MyHandler;

    #[async_trait::async_trait]
    impl EventsLoopTick<String> for MyHandler {
        async fn started(&self) {
            println!("loop started");
        }
        async fn tick(&self, model: String) -> RepeatIteration<String> {
            println!("got message: {model}");
            // The event is served - the loop may go for the next one.
            // To get one more iteration with it: RepeatIteration::Yes(model)
            RepeatIteration::No
        }
        async fn finished(&self) {
            println!("loop finished");
        }
    }

    // 2. Keep it inside AppCtx without any outer Mutex.
    pub struct AppCtx {
        pub events_loop: EventsLoop<String>,
    }

    impl AppCtx {
        pub fn new(
            app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
            logger: Arc<dyn Logger + Send + Sync + 'static>,
        ) -> Self {
            Self {
                events_loop: EventsLoop::new("my-loop", app_states, logger),
            }
        }
    }

    // 3. Wire up at startup.
    pub fn bootstrap(ctx: Arc<AppCtx>) {
        ctx.events_loop.register_event_loop(Arc::new(MyHandler));
        ctx.events_loop.start();
    }

    // 4. Produce messages from anywhere — `send` takes `&self` and never locks.
    pub fn produce(ctx: &AppCtx) {
        ctx.events_loop.send("hello".to_string());
    }
}
```

#### Detached publisher

For producers that don't need access to the whole `EventsLoop` (e.g. a background task, an HTTP handler held in its own struct), grab a cheap reference-counted publisher:

```rust
use rust_extensions::events_loop::EventsLoopPublisher;

let publisher: EventsLoopPublisher<String> = ctx.events_loop.get_publisher();

// Move/clone the publisher into other tasks; `send` / `stop` work the same way and stay lock-free.
tokio::spawn({
    let publisher = publisher.clone();
    async move {
        publisher.send("from background task".into());
    }
});
```

`get_publisher` returns `EventsLoopPublisher<TModel>` by value — every call hands out a clone which shares the one `Sender` created in `EventsLoop::new`. The `Sender` sits behind an `Arc` inside the publisher, so neither `get_publisher` nor `publisher.clone()` allocates anything.

Key properties:

- **Lock-free `send` / `stop`** — the `Sender` lives inside the shared `EventsLoopPublisher`; `.lock()` is only ever taken in `register_event_loop` and `start`.
- **One-shot registration** — a second `register_event_loop` panics; `start` without a prior register panics.
- **Bounded lifecycle** — `stop` delivers `Shutdown` through the same channel, so in-flight messages ahead of it are processed first.
- **Follows the application states** — that is what the `app_states` given to `new` is for. The reader waits for `is_initialized()` (checked once a second) before it calls `started()` and serves the first event; events sent meanwhile are not lost, they wait in the channel. `is_shutting_down()` is checked each time the loop goes for the next event, and before an iteration is repeated on `Yes(model)`: once it is raised the loop is over and `finished()` is called. A loop parked on an empty channel does not watch the flag — it wakes on the next event, serves it and only then leaves, while `stop()` wakes it right away. Once the loop is over the receiver is gone: `send` and `stop` panic from then on.
- **Per-tick timeout** — `set_iteration_timeout(Duration)` caps a single `tick` call; overruns are logged via the provided `Logger` and the loop keeps running.
- **The event is moved into the tick, and comes back to repeat it** — `tick` takes `TModel` by value: nothing is cloned and nothing is borrowed, so `TModel` only has to be `Send`. An iteration which is not done with the event returns `RepeatIteration::Yes(model)` — the reader starts a new iteration **with that very model** and a fresh timeout window, the way to do a long job in portions. `RepeatIteration::No` consumes the model and the loop goes for the next event.
- **A panic / a timeout drops the event** — the model was moved into the running future, so unwinding (or the timeout dropping that future) takes the model with it: there is nothing left to repeat with. Both cases are logged as an error via the provided `Logger` and the loop moves on to the next event instead of getting stuck. An event which must survive a failing tick has to be recoverable by the tick itself (catch the error inside `tick` and answer `Yes(model)`), or be held behind an `Arc` / a re-readable source.

### `BackgroundExecutor` use case

`BackgroundExecutor` offloads work off the caller's thread and finishes it on a single background Tokio task. The caller just signals "there may be work to do" via `trigger()` and returns immediately; the registered job runs in the background, decides for itself whether anything actually needs doing, and so a `trigger()` is allowed to fire idly.

A typical example is on-demand persistence: callers mutate state and `trigger()`; the job locks the shared state, takes whatever is pending, and persists it. If nothing changed since the last run the job locks, sees an empty set, and returns — a cheap no-op.

1. **Construct** — `BackgroundExecutor::new(name, logger)`; the name is used in panic and log messages.
2. **Register a job** (`BackgroundJob`) via `register` — one-shot; a second call panics.
3. **Start** — `start()` moves the registered job into the live state. Calling `start` without a prior `register` panics.
4. **Trigger** — `trigger()` adds one permit to a semaphore and returns. The reader task is already up (it was spawned by `start`), so a trigger spawns nothing, locks nothing and awaits nothing.

```rust
#[cfg(feature = "with-tokio")]
mod example {
    use std::sync::Arc;
    use rust_extensions::{
        background_executor::{BackgroundExecutor, BackgroundJob, RepeatIteration},
        Logger,
    };

    // 1. Define the work (no payload — the job pulls what it needs itself).
    struct FlushJob;

    #[async_trait::async_trait]
    impl BackgroundJob for FlushJob {
        async fn execute(&self) -> RepeatIteration {
            // lock shared state, take what is pending, persist it (no-op if nothing changed).

            // Still more to flush, but we do not want to keep this iteration
            // running — leave it and ask to be started again.
            RepeatIteration::Yes
        }
    }

    // Keep it inside AppCtx as a plain field.
    pub struct AppCtx {
        pub flush: BackgroundExecutor,
    }

    impl AppCtx {
        pub fn new(logger: Arc<dyn Logger + Send + Sync + 'static>) -> Self {
            Self {
                flush: BackgroundExecutor::new("flush", logger),
            }
        }
    }

    // 2 + 3. Wire up at startup.
    pub fn bootstrap(ctx: &AppCtx) {
        ctx.flush.register(Arc::new(FlushJob));
        ctx.flush.start();
    }

    // 4. Signal "there may be work" from anywhere — `trigger` takes `&self` and returns at once.
    pub fn on_change(ctx: &AppCtx) {
        ctx.flush.trigger();
    }
}
```

Key properties:

- **Offloads the caller** — `trigger()` returns immediately; the job runs on a dedicated background task, so the calling thread is never blocked on the work.
- **Single consumer** — at most one reader task is alive, so `execute()` never runs concurrently with itself.
- **Idle triggers are fine** — `execute()` runs once per `trigger()` and is expected to be cheap when there is nothing to do; the reader serves the permits one by one and parks once they are used up.
- **The job can ask for another iteration** — `execute()` returns `RepeatIteration`. `No` means "this iteration is done": the trigger being served is consumed, and the reader goes for the next permit. `Yes` means "there is more to do, but not in this call": the reader runs `execute()` again **without consuming the trigger**, so the work continues on a fresh iteration and no trigger is lost. That is the way out for a job which discovers mid-flight that it needs another pass and would rather leave the iteration than sit inside one long call — an external timeout, a batch limit, a fairness cap. A job that always answers `Yes` never lets the reader move on, exactly as an `execute()` that never returns would; the decision to stop belongs to the job.
- **Lock-free hot path** — `trigger()` takes no lock at all: it adds one permit to the semaphore and returns. `BackgroundExecutor` holds no mutex whatsoever — the job is registered into a `OnceLock`.
- **`trigger()` is legal from any thread** — a Tokio one, a plain `std::thread`, or an OS thread owned by a C++ host calling in through FFI. The reader is spawned once, by `start`, so a trigger never touches the runtime: `Semaphore::add_permits` is a plain synchronous method. **Only `start` has to be called from inside a runtime.** Permits accumulate, so N triggers are N iterations no matter how they interleave with the reader, and a trigger which arrives before the reader gets going is simply served later.
- **Panic-safe** — a panicking `execute()` is caught, logged via the provided `Logger`, and the reader keeps serving. A panic answers nothing, so it consumes the trigger like a `No` — a job that panics every time cannot spin the reader forever.
- **At work from `start()` on** — there is no `app_states` here: triggers are served from the moment `start` is called, whatever state the application is in, and for as long as the runtime lives. Start it once the application is ready for the job to run.
- **One-shot lifecycle** — a second `register` panics, `start` without a prior `register` panics, and `trigger` before `start` panics.

### `BackgroundExecutorWithMultiThreads` use case

`BackgroundExecutorWithMultiThreads<TThreadId>` is the same component, but the work is split into independent threads by the `thread_id` given to `trigger()`. The rule is: **one thread id — one background task**. Triggers of the same thread id are served strictly one by one, triggers of different thread ids are served in parallel, and the thread id itself is handed to `execute()` so a single job instance can serve all of them.

That is the shape for per-entity work: flush the state of an account, of a trading instrument, of a connection. Entities must not step on each other's toes, yet each of them alone has to be persisted in the order the changes happened.

The lifecycle is the same as of `BackgroundExecutor` — `new(name, logger)` → `register` → `start()` → `trigger(thread_id)` — with one addition: the background task of a thread id is created by the trigger which created the thread, and it is **removed** as soon as the triggers of that thread id are drained. Nothing is kept alive for an idle thread id, and the next trigger of it spawns a fresh task.

```rust
#[cfg(feature = "with-tokio")]
mod example {
    use std::sync::Arc;
    use rust_extensions::{
        background_executor_with_multi_threads::{
            BackgroundExecutorWithMultiThreads, BackgroundJobWithMultiThreads, RepeatIteration,
        },
        Logger,
    };

    // 1. Define the work. The same job instance serves every thread id.
    struct FlushAccountJob;

    #[async_trait::async_trait]
    impl BackgroundJobWithMultiThreads<u64> for FlushAccountJob {
        async fn execute(&self, account_id: &u64) -> RepeatIteration {
            // take what is pending for this account only and persist it.
            let _ = account_id;
            RepeatIteration::No
        }
    }

    // Keep it inside AppCtx as a plain field.
    pub struct AppCtx {
        pub flush: BackgroundExecutorWithMultiThreads<u64>,
    }

    impl AppCtx {
        pub fn new(logger: Arc<dyn Logger + Send + Sync + 'static>) -> Self {
            Self {
                flush: BackgroundExecutorWithMultiThreads::new("flush-accounts", logger),
            }
        }
    }

    // 2 + 3. Wire up at startup.
    pub fn bootstrap(ctx: &AppCtx) {
        ctx.flush.register(Arc::new(FlushAccountJob));
        ctx.flush.start();
    }

    // 4. Signal "there may be work" for a certain account — returns at once.
    pub fn on_account_changed(ctx: &AppCtx, account_id: u64) {
        ctx.flush.trigger(account_id);
    }
}
```

Key properties on top of the ones `BackgroundExecutor` gives:

- **Sequential per thread id** — every thread id has at most one reader task alive, so two iterations of the same id never overlap and they run in the order the triggers arrived.
- **Parallel across thread ids** — readers of different thread ids are independent Tokio tasks; a slow id does not hold the others back.
- **The id is a part of the callback** — `execute(&self, thread_id: &TThreadId)`, so one registered job serves all the threads and decides what to do out of the id.
- **Tasks are created and removed within the thread id** — the trigger which creates a thread id spawns its reader; the reader which drains the counter of that thread id removes it and exits. Both happen under one lock, so a trigger arriving at a draining thread either lands in the still-alive reader or spawns a new one — it is never lost. This is why the thread ids may be unbounded (an account id, an instrument): nothing is kept alive for an idle one.
- **`trigger()` is legal from any thread** — same as for `BackgroundExecutor`, including an OS thread owned by a C++ host. Here the readers do come and go with the thread ids, so the runtime to spawn them on is captured by `start` and used from then on; a trigger never needs a runtime of its own. **Only `start` has to be called from inside a runtime.**
- **Any id type** — `TThreadId: Hash + Eq + Clone + Send + Sync + 'static` (`u64`, `String`, `Arc<String>`, a tuple key, …).
- **`get_working_threads_amount()`** — how many thread ids have a reader alive right now.

### `IsInitialized` use case

`IsInitialized` is a one-shot initialization gate. Any number of tasks can `await` `wait_until_initialized`, and they all stay parked until initialization happens exactly once. It is designed to live inside an `AppCtx` as a plain field — all methods take `&self`, no outer `Mutex` needed.

The flow:

1. **Construct** — `IsInitialized::new()` (or `Default::default()`); starts un-initialized.
2. **Wait** — callers `wait_until_initialized().await`. While un-initialized, each subscribes (a `TaskCompletion` is parked in an internal `Vec`); once initialized, the call returns instantly via an `AtomicBool` fast path without touching the mutex.
3. **Initialize** — `initialized().await` raises the flag and completes every parked awaiter, releasing them all at once.
4. **Guard** — `panic_if_not_initialized()` panics with `"Not Initialized"` if called before initialization; `wait_some_time_and_panic(duration)` awaits initialization for at most `duration` and panics with `"Not Initialized"` if it does not arrive in time; `is_initialized()` returns the flag.

```rust
#[cfg(feature = "with-tokio")]
mod example {
    use std::sync::Arc;
    use rust_extensions::IsInitialized;

    // Keep it inside AppCtx as a plain field.
    pub struct AppCtx {
        pub is_initialized: IsInitialized,
    }

    impl AppCtx {
        pub fn new() -> Self {
            Self {
                is_initialized: IsInitialized::new(),
            }
        }
    }

    // Any number of tasks can park here until initialization happens.
    pub async fn handle_request(ctx: Arc<AppCtx>) {
        ctx.is_initialized.wait_until_initialized().await;
        // ... proceed, the app is ready ...
    }

    // Called once, when bootstrap finished wiring everything up.
    pub async fn on_bootstrap_finished(ctx: &AppCtx) {
        ctx.is_initialized.initialized().await;
    }
}
```

Key properties:

- **Lock-free fast path** — once initialized, `wait_until_initialized` only reads an `AtomicBool` and returns; the `tokio::Mutex` is entered only while still un-initialized.
- **No lost wake-ups** — the slow path re-checks the flag *after* taking the mutex, and `initialized` raises the flag and drains the waiter `Vec` under that same mutex, so a caller either observes the flag or is guaranteed to be completed.
- **Idempotent `initialized`** — calling it again is a no-op (the waiter `Vec` is already drained).
- **Cancel-safe** — if a waiter's future is dropped before completion, `initialized` skips the dead subscription without panicking.

### Idempotency caches use case

An idempotency cache makes a retried request execute **at most once**. There are two flavours, one per module, built on the same machinery:

| Module | A request is identified by | Use it when |
| --- | --- | --- |
| `idempotency::by_process_id` | `process_id: TProcessId` | the process id is unique on its own — typically the client's request id |
| `idempotency::by_user_id_and_process_id` | `user_id: TUserId` + `process_id: TProcessId` | a process id is only unique within its user — the same process id of two users is two independent requests |

The ids are generics which only have to be comparable for equality: `PartialEq` is all it takes (plus `Send + Sync + 'static`) — they are never hashed, ordered or cloned. Each module has its own `IdempotencyCache` and `IdempotencyExecution` trait, and re-exports the shared `IdempotencyResult`, `DEFAULT_MAX_AMOUNT` and `DEFAULT_EXECUTION_TIMEOUT`, so one `use` of the module is enough. The execution gets its id(s) by reference, so it can act on behalf of the user (or store the process id next to the side effect) without the caller copying the ids into `params`.

For a given id (pair of ids):

- **first call** — runs `IdempotencyExecution::execute` inline and memorizes its `Result`;
- **a retry that arrives while the first call is still running** — executes nothing: it parks on a `TaskCompletion` and is released with the very same result;
- **a retry that arrives after it finished** — gets the memorized result immediately, the execution is not touched.

Like `EventsLoop` and `BackgroundExecutor`, a cache is designed to live inside an `AppCtx` as a plain field (all methods take `&self`), and the execution is registered separately so it is free to hold an `Arc` of the `AppCtx` that owns the cache.

Keyed by a process id:

```rust
#[cfg(feature = "with-tokio")]
mod example {
    use std::sync::Arc;
    use std::time::Duration;
    use rust_extensions::idempotency::by_process_id::{
        IdempotencyCache, IdempotencyExecution, DEFAULT_MAX_AMOUNT,
    };

    pub struct ChargeParams {
        pub client_id: String,
        pub amount: f64,
    }

    // 1. Define the work that must not happen twice.
    struct ChargeExecution;

    #[async_trait::async_trait]
    impl IdempotencyExecution<String, ChargeParams, String, String> for ChargeExecution {
        async fn execute(&self, process_id: &String, params: ChargeParams) -> Result<String, String> {
            // Runs exactly once per process id.
            Ok(format!("{}: charged {} for {}", process_id, params.amount, params.client_id))
        }
    }

    // 2. Keep it inside AppCtx as a plain field.
    pub struct AppCtx {
        pub charges: IdempotencyCache<String, ChargeParams, String, String>,
    }

    impl AppCtx {
        pub fn new() -> Self {
            Self {
                charges: IdempotencyCache::new_with_max_amount("charges", DEFAULT_MAX_AMOUNT)
                    .set_execution_timeout(Duration::from_secs(5)),
            }
        }
    }

    // 3. Wire up at startup.
    pub fn bootstrap(ctx: &AppCtx) {
        ctx.charges.register_execution(Arc::new(ChargeExecution));
    }

    // 4. Handle a request — retrying it with the same process id never charges twice.
    pub async fn handle_request(ctx: &AppCtx, process_id: String, params: ChargeParams) {
        match ctx.charges.execute(process_id, params).await {
            Ok(receipt) => println!("{}", receipt.as_str()),
            Err(err) => println!("failed: {}", err.as_str()),
        }
    }
}
```

Keyed by a user id + process id — the wiring is the same, only the key and the execution signature differ:

```rust
#[cfg(feature = "with-tokio")]
mod example_by_user {
    use std::sync::Arc;
    use rust_extensions::idempotency::by_user_id_and_process_id::{
        IdempotencyCache, IdempotencyExecution,
    };

    pub struct WithdrawalParams {
        pub amount: f64,
    }

    struct WithdrawalExecution;

    #[async_trait::async_trait]
    impl IdempotencyExecution<i64, String, WithdrawalParams, String, String> for WithdrawalExecution {
        async fn execute(
            &self,
            user_id: &i64,
            process_id: &String,
            params: WithdrawalParams,
        ) -> Result<String, String> {
            // Runs exactly once per (user_id, process_id), on behalf of that user.
            Ok(format!("{}: user {} withdrew {}", process_id, user_id, params.amount))
        }
    }

    pub struct AppCtx {
        pub withdrawals: IdempotencyCache<i64, String, WithdrawalParams, String, String>,
    }

    impl AppCtx {
        pub fn new() -> Self {
            Self {
                withdrawals: IdempotencyCache::new("withdrawals"),
            }
        }
    }

    pub fn bootstrap(ctx: &AppCtx) {
        ctx.withdrawals.register_execution(Arc::new(WithdrawalExecution));
    }

    // A retry of the same user's process id gets the memorized answer, while another
    // user with the very same process id is executed on its own.
    pub async fn handle_request(
        ctx: &AppCtx,
        user_id: i64,
        process_id: String,
        params: WithdrawalParams,
    ) {
        match ctx.withdrawals.execute(user_id, process_id, params).await {
            Ok(receipt) => println!("{}", receipt.as_str()),
            Err(err) => println!("failed: {}", err.as_str()),
        }
    }
}
```

Key properties (both flavours):

- **At most one execution per id** — concurrent retries park on the first one instead of starting their own; only the caller that actually executes consumes its `params`, the others simply drop theirs.
- **Errors are memorized too** — once an id produced an answer, every retry of that id gets that answer back, `Ok` or `Err` alike. There is no "retry the failure for free": a genuinely new attempt needs a new process id.
- **Shared as `Arc`** — the result is handed out as `Result<Arc<TOk>, Arc<TErr>>` (`IdempotencyResult`), so serving N retries costs N atomic increments, and neither `TOk` nor `TErr` has to be `Clone`.
- **Last N, FIFO** — the last `max_amount` results are kept (`new` uses `DEFAULT_MAX_AMOUNT` = 1000, `new_with_max_amount` sets it), evicted oldest-completed-first; a cache hit does **not** refresh an entry. `max_amount == 0` is legal and means "de-duplicate concurrent retries, remember nothing afterwards". In `by_user_id_and_process_id` the cap is shared by all users — it is not a per-user quota.
- **One flat queue, no index** — the whole state is a single `VecDeque`: push new ids to the back, drop the oldest results from the front, look up by linear scan — which is exactly why `PartialEq` is all an id needs. There is no second structure that could drift out of sync with it. Eviction steps **over** in-flight entries rather than dropping the front blindly — an `Executing` entry has awaiters parked on it — so `max_amount` caps the memorized answers and in-flight executions sit on top of that. The linear scan is the right shape at these sizes, not for a `max_amount` in the hundreds of thousands.
- **Cancel-safe by design, loud about it** — the first caller owns the execution, so if its future is dropped (HTTP timeout) or the execution panics, a drop-guard removes the entry — the next retry executes from scratch — and everybody parked on it gets the standard `TaskCompletion` drop behaviour: their `get_result()` panics with `"Task is dropped"`. Nothing is memorized in that case, because we do not know whether the side effect happened.
- **Bounded execution** — `execute` is wrapped in a timeout (`DEFAULT_EXECUTION_TIMEOUT` = 5s, builder `set_execution_timeout`). Overrunning it is simply the third way to not produce a result, so it is handled as a panic like the other two. Without it a hung execution would pin its id forever and every retry of that id would park forever, since an `Executing` entry is never evicted. Needs a Tokio runtime with time enabled.
- **No lock held across `.await`** — a `parking_lot::Mutex` guards the queue (lookup / push / remove only); the execution and every completion happen outside it. `parking_lot` is also what makes the synchronous cancellation drop-guard possible.
- **One-shot registration** — a second `register_execution` panics, and `execute` before registration panics (before it claims the id, so no entry is leaked). The registered handler lives in a `OnceLock`, so reading it on every `execute` is a single atomic load that hands back a reference — the hot path never touches the `Arc` refcount.

### `MyExactTimer` use case

`MyExactTimer` is the drift-free sibling of `MyTimer`. Where `MyTimer` sleeps `interval` *between* ticks (so ticks slowly drift and a slow tick pushes every later one back), `MyExactTimer` fires exactly on the **aligned wall-clock marks** of a fixed `ExactTimerInterval` — e.g. `Every5Seconds` fires at seconds `:00, :05, :10, … :55`, and `Every5Minutes` at minutes `:00, :05, … :55`.

It reuses the exact same `MyTimerTick` trait, so an existing tick can be registered on either timer — you only swap which timer you register it on.

```rust
use std::sync::Arc;
use rust_extensions::{MyExactTimer, ExactTimerInterval, MyTimerTick, RepeatTimerIteration};

struct MyTick;

#[async_trait::async_trait]
impl MyTimerTick for MyTick {
    async fn tick(&self) -> RepeatTimerIteration {
        // runs at :00, :05, :10 … of every minute
        RepeatTimerIteration::WithInterval
    }
}

pub fn bootstrap(logger: Arc<dyn rust_extensions::Logger + Send + Sync + 'static>) {
    let mut timer = MyExactTimer::new(ExactTimerInterval::Every5Seconds, logger);
    timer.register_timer("my-tick", Arc::new(MyTick));
    timer.start();
}
```

Available intervals: `Every1Second`, `Every5Seconds`, `Every10Seconds`, `Every15Seconds`, `Every20Seconds`, `Every30Seconds`, `Every1Minute`, `Every5Minutes`, `Every10Minutes`, `Every15Minutes`, `Every20Minutes`, `Every30Minutes`.

How it stays exact:

- **Epoch-aligned marks** — the next fire time is the next multiple of the interval since the Unix epoch. Because the epoch sits on a minute/hour boundary and every interval evenly divides a minute or an hour, those multiples land precisely on the natural wall-clock marks. No accumulated drift.
- **Recomputed after every tick** — the next mark is computed from the moment the tick *finished*, so a slow tick simply skips to the next mark instead of pushing the whole schedule back. Finishing exactly on a mark advances to the following one (never a double fire).
- **Coarse-to-fine wait** — the timer approaches the mark by sleeping in shrinking chunks (`10s → 5s → 1s`), re-measuring each loop; once under one second remains it does a single exact sleep and wakes right on the mark. A long interval therefore never goes more than 10 seconds without looking at the wall clock.
- **Same lifecycle as `MyTimer`** — ticks from `start()` on, for as long as the runtime lives (neither timer watches the application states — start it once the application is ready to be ticked), supports multiple registered ticks (fired together on each mark), a per-iteration timeout (`new_with_execute_timeout` / `set_iteration_timeout`, default 60s), and panic-catching that logs via the provided `Logger`.
- **One loop per timer** — on either timer `start()` is called once: a second call panics (`… is already started`, naming the registered ticks and the interval) rather than putting a second loop on the same ticks, which would run each of them twice. Register every tick before `start()` — the loop works with the ticks registered by then.

### `RepeatTimerIteration` — leaving a tick early to reset the timeout

Both timers wrap every `tick()` in `iteration_timeout` (`new_with_execute_timeout` / `set_iteration_timeout`, default 60s). A tick with more work than fits in that window does not have to race it: it returns **`RepeatTimerIteration::Immediately`** and is started again straight away — **with the timeout window reset** — instead of being cut off mid-flight. `WithInterval` is the normal answer: the iteration is done, wait for the next scheduled tick.

```rust
#[async_trait::async_trait]
impl MyTimerTick for FlushTick {
    async fn tick(&self) -> RepeatTimerIteration {
        let batch = take_next_batch().await;

        if batch.is_empty() {
            return RepeatTimerIteration::WithInterval;
        }

        flush(batch).await;

        // More is waiting — leave this iteration and get a fresh timeout budget
        // for the next portion, rather than doing it all in one long call.
        RepeatTimerIteration::Immediately
    }
}
```

- **Only the tick that asked is repeated** — with several ticks registered on one timer, a neighbour that answered `WithInterval` keeps its own schedule and is not dragged into the extra passes.
- **A panic or a timeout answered nothing** — neither is repeated, so a tick that always panics cannot spin the loop.
- **The schedule does not shift** — on `MyExactTimer` the next mark is computed once the extra passes are finished, exactly as it is after a single slow tick; on `MyTimer` the interval is slept once they are done.
- **A tick that always answers `Immediately` never lets the timer sleep** — same as one that never returns; the decision to stop belongs to the tick.
- **`execute_timer(name)`** (the manual, out-of-schedule call on either timer) has no interval to wait for, so it hands the `RepeatTimerIteration` back to the caller instead of acting on it.

## IO, logging, misc

- `file_utils`: path helpers — `format_path` expands `~` into `$HOME`, `FilePath` builds a path by appending and removing segments.
- `logger`: simple structured logger traits.
- `remote_endpoint`: parse an address into scheme, host, port and HTTP path with query — `RemoteEndpoint` (`http`, `https`, `ws`, `wss` and unix-socket addresses), `SshRemoteEndpoint` for `ssh://user@host:port`, `RemoteEndpointHostString` for a host reached through SSH (`ssh://user@host:port->host:port`).
- Math/min-max helpers: `min_value`, `max_value`, `min_key_value`, `max_value`.
- `StopWatch` / `AtomicStopWatch` / `AtomicDuration` for timing.

## Development status

- License: MIT (see `LICENSE.md`).
- Current crate version: **0.1.5**.
- Contributions are welcome via pull requests.

