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
- Async/Tokio (feature `with-tokio`): `events_loop`, `background_executor`, `my_timer`, `task_completion`, `tokio_queue`, `queue_to_save`, `queue_to_save_with_id`, `application_states`, `sortable_id`.
- IO & misc: `file_utils`, `remote_endpoint`, `logger`, `min_value`, `max_value`, `min_key_value`, `placeholders`, `maybe_short_string`.

## Quick recipes

- Time point + interval keys:
  - `date_time::DateTimeAsMicroseconds` for UTC timestamps with µs precision.
  - `date_time::interval_key::*` for rounding/grouping into year/month/week/day/hour (1h/2h/4h)/minute (1m/5m/15m/30m) buckets.
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
  - `MyTimer` for tick-driven tasks, `EventsLoop` for fan-out processing, `BackgroundExecutor` to offload bursty work onto a single background task, `TaskCompletion` for awaiting completion handles, `TokioQueue` for bounded async queues, `QueueToSave` for producer/consumer disk pipelines, `ApplicationStates` for async state transitions.
- File/IO:
  - `file_utils::read_file_lines_iter`, `array_of_bytes_iterator::FileIterator`, `remote_endpoint` helpers for host/port parsing.

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

- `EventsLoop`: single-consumer async message loop — `send` is lock-free, the consumer runs in a dedicated Tokio task.
- `BackgroundExecutor`: offloads work from the caller onto a single background Tokio task — `trigger()` is lock-free in steady state and runs the registered `execute()` exactly once per call, never in parallel.
- `MyTimer`: tick-based scheduling with graceful stop.
- `TaskCompletion`: create awaitable completion sources with error support.
- `TokioQueue`: bounded async queue with backpressure.
- `QueueToSave`: producer/consumer file-saving pipeline with retries.
- `QueueToSaveWithId`: same producer/consumer batching as `QueueToSave`, but each item implements `PersistObjectId<ID>`. Re-enqueuing an item with an ID already in the queue overwrites the pending entry, so only the latest state per ID is flushed to the handler. `ID` must be `Hash + Eq + Clone`; the handler receives a `Vec<T>` per tick. No ordering guarantee across IDs.
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

### `EventsLoop` use case

`EventsLoop` is designed to live inside an `AppCtx` as a plain field (no outer `Mutex` / no `mut` access needed). The flow:

1. **Construct in `AppCtx::new`** — the channel is created immediately, so `send` is available right away and is lock-free (no mutex on the hot path).
2. **Register a callback** (`EventsLoopTick`) via `register_event_loop` — typically during app initialization, once dependencies are wired.
3. **Start** — spawns the background reader task which owns the receiver + callback and drives `started` / `tick` / `finished`.
4. **Send / stop** — `send(msg)` pushes a message; `stop()` sends a shutdown signal.

```rust
#[cfg(feature = "with-tokio")]
mod example {
    use std::sync::Arc;
    use rust_extensions::{
        events_loop::{EventsLoop, EventsLoopTick},
        ApplicationStates, Logger,
    };

    // 1. Define what each tick should do.
    struct MyHandler;

    #[async_trait::async_trait]
    impl EventsLoopTick<String> for MyHandler {
        async fn started(&self) {
            println!("loop started");
        }
        async fn tick(&self, model: String) {
            println!("got message: {model}");
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
        pub fn new() -> Self {
            Self {
                events_loop: EventsLoop::new("my-loop"),
            }
        }
    }

    // 3. Wire up at startup.
    pub async fn bootstrap(
        ctx: Arc<AppCtx>,
        app_states: Arc<dyn ApplicationStates + Send + Sync + 'static>,
        logger: Arc<dyn Logger + Send + Sync + 'static>,
    ) {
        ctx.events_loop
            .register_event_loop(Arc::new(MyHandler))
            .await;
        ctx.events_loop.start(app_states, logger).await;
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
use std::sync::Arc;
use rust_extensions::events_loop::EventsLoopPublisher;

let publisher: Arc<EventsLoopPublisher<String>> = ctx.events_loop.get_publisher();

// Move/clone the Arc into other tasks; `send` / `stop` work the same way and stay lock-free.
tokio::spawn({
    let publisher = publisher.clone();
    async move {
        publisher.send("from background task".into());
    }
});
```

`get_publisher` returns `Arc<EventsLoopPublisher<TModel>>` — every call hands out a clone of the same shared publisher (the `Sender` is created once in `EventsLoop::new`).

Key properties:

- **Lock-free `send` / `stop`** — the `Sender` lives inside the shared `EventsLoopPublisher`; `.lock()` is only ever taken in `register_event_loop` and `start`.
- **One-shot registration** — a second `register_event_loop` panics; `start` without a prior register panics.
- **Bounded lifecycle** — `stop` delivers `Shutdown` through the same channel, so in-flight messages ahead of it are processed first.
- **Per-tick timeout** — `set_iteration_timeout(Duration)` caps a single `tick` call; overruns are logged via the provided `Logger` and the loop keeps running.

### `BackgroundExecutor` use case

`BackgroundExecutor` offloads work off the caller's thread and finishes it on a single background Tokio task. The caller just signals "there may be work to do" via `trigger()` and returns immediately; the registered job runs in the background, decides for itself whether anything actually needs doing, and so a `trigger()` is allowed to fire idly.

A typical example is on-demand persistence: callers mutate state and `trigger()`; the job locks the shared state, takes whatever is pending, and persists it. If nothing changed since the last run the job locks, sees an empty set, and returns — a cheap no-op.

1. **Construct** — `BackgroundExecutor::new(name)`; the name is used in panic and log messages.
2. **Register a job** (`BackgroundJob`) via `register` — one-shot; a second call panics.
3. **Start** — `start(logger)` moves the registered job into the live state. Calling `start` without a prior `register` panics.
4. **Trigger** — `trigger()` bumps an atomic counter; the first trigger (0 → 1) spawns the reader task, and further triggers while the reader is alive just increment the counter and stay lock-free.

```rust
#[cfg(feature = "with-tokio")]
mod example {
    use std::sync::Arc;
    use rust_extensions::{
        background_executor::{BackgroundExecutor, BackgroundJob},
        Logger,
    };

    // 1. Define the work (no payload — the job pulls what it needs itself).
    struct FlushJob;

    #[async_trait::async_trait]
    impl BackgroundJob for FlushJob {
        async fn execute(&self) {
            // lock shared state, take what is pending, persist it (no-op if nothing changed).
        }
    }

    // Keep it inside AppCtx as a plain field.
    pub struct AppCtx {
        pub flush: BackgroundExecutor,
    }

    impl AppCtx {
        pub fn new() -> Self {
            Self {
                flush: BackgroundExecutor::new("flush"),
            }
        }
    }

    // 2 + 3. Wire up at startup.
    pub fn bootstrap(ctx: &AppCtx, logger: Arc<dyn Logger + Send + Sync + 'static>) {
        ctx.flush.register(Arc::new(FlushJob));
        ctx.flush.start(logger);
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
- **Idle triggers are fine** — `execute()` runs once per `trigger()` and is expected to be cheap when there is nothing to do; the reader drains the counter back to zero before exiting.
- **Lock-free hot path** — `trigger()` only takes a lock on the 0 → 1 transition (to spawn the reader); subsequent triggers are a single atomic add.
- **Panic-safe** — a panicking `execute()` is caught, logged via the provided `Logger`, and the loop keeps draining.
- **One-shot lifecycle** — a second `register` panics, `start` without a prior `register` panics, and `trigger` before `start` panics.

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

