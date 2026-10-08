# rust-extensions — index

The base crate of the MyJetTools stack: time, strings, binary encoding, collections and Tokio primitives.

Read a topic: `resource://rust-extensions/{topic}`, or `get_rust_extensions_readme` with `topic`. Every topic has contracts and compiled examples for each case.

```toml
rust-extensions = { tag = "${last_tag}", git = "https://github.com/MyJetTools/rust-extensions.git", features = ["with-tokio"] }
```

| Feature | Enables |
| --- | --- |
| `with-tokio` | Timers, events loop, background executors, save queues, app lifecycle, `TaskCompletion`, `IsInitialized`, `TokioQueue`. Implies `rnd`. |
| `rnd` | `uuid::generate_v4()`, `SortableId`. |
| `base64`, `hex` | Encoding helpers. |
| `objects-pool` | `ObjectsPool`. |
| `vec-maybe-stack` | `VecMaybeStack`. |

Builds for `wasm32`; timers, queues, executors, signals and pooling are compiled out there.

**Startable** lists the types that implement `Startable`: they do nothing until `start()` is called, exactly once — see `app-lifecycle`.

| Topic | What is inside | Startable |
| --- | --- | --- |
| [`date-time`](date-time.md) | `DateTimeAsMicroseconds` — UTC µs timestamp, parsing, formats, serde; `DateTimeStruct` calendar fields, month steps; showing an instant in a time zone (fixed offset, readable `YYYY-MM-DD HH:MM:SS`); reading a time the user typed | — |
| [`interval-keys`](interval-keys.md) | Bucket timestamps into minute…year keys stored as sortable `i64` | — |
| [`durations`](durations.md) | Parse / print `Duration`, `StopWatch`, `AtomicStopWatch`, `AtomicDuration` | — |
| [`strings`](strings.md) | `ShortString`, `MaybeShortString`, `StrOrString`, `StringBuilder`, `AsStr`, case-insensitive helpers | — |
| [`secure-string-builder`](secure-string-builder.md) | Build secrets without leaving copies in freed memory | — |
| [`binary`](binary.md) | `BinaryPayloadBuilder`, varint, `SliceOrVec`, byte search and cursors, `AsyncBytesStream` — a byte stream read in chunks, `BufferedReader` — parsing it across the chunks, `DoubleBuffer` — two buffers between the task which reads and the task which parses, `binary_search`, hex, base64 | — |
| [`sorted-vec`](sorted-vec.md) | Vectors kept sorted by a key from the item — one key, string key, two string keys, `Arc` flavours | — |
| [`collections`](collections.md) | Grouping, lazy containers, `linq`, auto-shrink, running min/max, `VecMaybeStack`, `ObjectsPool` | — |
| [`sized-chunks`](sized-chunks.md) | Batch by measured byte size instead of item count | — |
| [`timers`](timers.md) | `MyTimer` every interval, `MyExactTimer` on wall-clock marks (`:00`, `:05`…); long jobs in portions | `MyTimer`, `MyExactTimer` |
| [`events-loop`](events-loop.md) | Single-consumer async message loop | `EventsLoop` |
| [`background-executor`](background-executor.md) | Trigger work onto a background task, globally or per thread id | `BackgroundExecutor`, `BackgroundExecutorWithMultiThreads` |
| [`queue-to-save`](queue-to-save.md) | Write-behind queues: single, bulk, latest-state-per-ID, upsert-or-delete | `QueueToSave`, `QueueToSaveAsBulk`, `QueueToSaveWithId`, `QueueToSaveOrDeleteWithId` |
| [`idempotency`](idempotency.md) | Execute a retried request at most once | — |
| [`async-primitives`](async-primitives.md) | `TaskCompletion`, `IsInitialized`, `TokioQueue` | — |
| [`app-lifecycle`](app-lifecycle.md) | `Logger`, `ApplicationStates` / `AppStates`, `Startable` | — |
| [`remote-endpoint`](remote-endpoint.md) | Parse `scheme://host:port/path`, scheme default ports, unix sockets, SSH and SSH-tunnelled addresses | — |
| [`misc`](misc.md) | Paths, uuid, `SortableId`, `DataWrapper`, re-exports | — |
