rust-extensions
===============

The base crate of the MyJetTools stack: small building blocks the other crates are built on — time, strings, binary encoding, collections and Tokio primitives.

## Install

```toml
[dependencies]
rust-extensions = { tag = "${last_tag}", git = "https://github.com/MyJetTools/rust-extensions.git", features = ["with-tokio"] }
```

| Feature | Enables |
| --- | --- |
| `with-tokio` | Async primitives: timers, events loop, background executors, save queues, app lifecycle. Implies `rnd`. |
| `rnd` | `uuid::generate_v4()`, `SortableId`. |
| `base64`, `hex` | Encoding helpers. |
| `objects-pool` | `ObjectsPool`. |
| `vec-maybe-stack` | `VecMaybeStack`. |

Builds for `wasm32`; native-only parts (timers, queues, executors, signals, pooling) are compiled out there.

## Topics

Each topic is a document in [`docs/`](docs). Its name is the file name, and also the parameter of the development-mcp resource: `resource://rust-extensions/{topic}`.

| Topic | What is inside |
| --- | --- |
| [`date-time`](docs/date-time.md) | `DateTimeAsMicroseconds` — UTC µs timestamp, parsing, formats, serde; time zones, client time |
| [`interval-keys`](docs/interval-keys.md) | Bucket timestamps into minute…year keys stored as sortable `i64` |
| [`durations`](docs/durations.md) | Parse / print `Duration`, `StopWatch`, `AtomicStopWatch`, `AtomicDuration` |
| [`strings`](docs/strings.md) | `ShortString`, `MaybeShortString`, `StrOrString`, `StringBuilder`, `AsStr`, case-insensitive helpers |
| [`secure-string-builder`](docs/secure-string-builder.md) | Build secrets without leaving copies in freed memory |
| [`binary`](docs/binary.md) | `BinaryPayloadBuilder`, varint, `SliceOrVec`, byte search and cursors, `binary_search`, hex, base64 |
| [`sorted-vec`](docs/sorted-vec.md) | Vectors kept sorted by a key from the item — one key, string key, two string keys, `Arc` flavours |
| [`collections`](docs/collections.md) | Grouping, lazy containers, `linq`, auto-shrink, running min/max, `VecMaybeStack`, `ObjectsPool` |
| [`sized-chunks`](docs/sized-chunks.md) | Batch by measured byte size instead of item count |
| [`timers`](docs/timers.md) | `MyTimer`, `MyExactTimer` — periodic work, long jobs in portions |
| [`events-loop`](docs/events-loop.md) | Single-consumer async message loop |
| [`background-executor`](docs/background-executor.md) | Trigger work onto a background task, globally or per thread id |
| [`queue-to-save`](docs/queue-to-save.md) | Write-behind queues: single, bulk, latest-state-per-ID, upsert-or-delete |
| [`idempotency`](docs/idempotency.md) | Execute a retried request at most once |
| [`async-primitives`](docs/async-primitives.md) | `TaskCompletion`, `IsInitialized`, `TokioQueue` |
| [`app-lifecycle`](docs/app-lifecycle.md) | `Logger`, `ApplicationStates` / `AppStates`, `Startable` |
| [`remote-endpoint`](docs/remote-endpoint.md) | Parse `scheme://host:port/path`, unix sockets, SSH and SSH-tunnelled addresses |
| [`misc`](docs/misc.md) | Paths, uuid, `SortableId`, `DataWrapper`, re-exports |

The examples in `docs/` are compiled and run by `cargo test --doc --all-features`.

## License

MIT — see [LICENSE.md](LICENSE.md).
