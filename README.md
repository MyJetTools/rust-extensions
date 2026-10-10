rust-extensions
===============

`rust-extensions` is the base crate of the MyJetTools stack. The other services and libraries build on it: it defines the timestamp type they exchange, the small string and binary types they pass around, the sorted containers behind in-memory caches, and the background machinery a service is assembled from — timers, an events loop, background executors and write-behind queues.

The dependencies stay light. The async part sits behind the `with-tokio` feature, and the crate builds for `wasm32`, so the same types work in a browser front-end.

## Install

```toml
[dependencies]
rust-extensions = { tag = "${last_tag}", git = "https://github.com/MyJetTools/rust-extensions.git", features = ["with-tokio"] }
```

| Feature | Enables |
| --- | --- |
| `with-tokio` | Timers, events loop, background executors, save queues, app lifecycle, `TaskCompletion`, `IsInitialized`, `TokioQueue`. Implies `rnd`. |
| `rnd` | `uuid::generate_v4()`, `SortableId`. |
| `base64`, `hex` | Encoding helpers. |
| `objects-pool` | `ObjectsPool`. |
| `vec-maybe-stack` | `VecMaybeStack`. |

On `wasm32` the parts that need the OS — timers, queues, executors, signals, pooling — are compiled out.

## What is inside

The crate is documented by topic. Each topic below links to a document in [`docs/`](docs) with the contracts and an example for every case. The examples are compiled and run by `cargo test --doc --all-features`, so they stay in step with the code.

### Time

- [**date-time**](docs/date-time.md) — `DateTimeAsMicroseconds`, the timestamp of the whole stack: a UTC instant stored as `i64` microseconds. It parses RFC 3339 and unix timestamps in any unit, renders the HTTP, e-mail and compact formats, and serializes to RFC 3339 while still reading the numbers older services stored. `DateTimeStruct` gives the calendar fields, and `DateTimeAsMicrosecondsWithTimeZone` shows an instant in the user's own zone.
- [**interval-keys**](docs/interval-keys.md) — cut a timestamp to its minute, hour, day, week, month or year and use the bucket as a compact `i64` key that sorts the way time does: candles, daily statistics, hourly reports.
- [**durations**](docs/durations.md) — parse durations from settings (`150ms`, `01:30`, `1d 00:00:01`), print them for logs, measure elapsed time, and keep a timeout that can be changed at runtime.

### Strings

- [**strings**](docs/strings.md) — `ShortString` keeps up to 255 bytes inline, with no heap allocation. `MaybeShortString` switches to a `String` once the value grows past that, and `StrOrString` holds a borrowed or an owned string behind one type. Also a string builder and case-insensitive helpers.
- [**secure-string-builder**](docs/secure-string-builder.md) — build a password, a token or a connection string so that no copy of it is left in freed memory: every outgrown buffer is zeroed before it is released.

### Bytes

- [**binary**](docs/binary.md) — write little-endian payloads and variable-size lengths, keep a borrowed or an owned buffer behind one type, search and walk through bytes in memory or in a file, read a byte stream chunk by chunk through `AsyncBytesStream` — a file through `FileStreamReader`, 64 KB a chunk — and parse it across the chunks with `BufferedReader`, read in one task and parse in another through the two buffers of `DoubleBuffer`, binary-search by key, encode hex and base64.
- [**sized-chunks**](docs/sized-chunks.md) — split a collection into batches by measured size, so a gRPC message or a request body stays under its byte limit however large the items are.

### Collections

- [**sorted-vec**](docs/sorted-vec.md) — a `Vec` kept sorted by a key taken from the item itself: binary-search lookups, ranges of keys as slices, entries that insert or update after a single search. There are variants with a string key, two string keys and `Arc` items. It is the in-memory index behind caches and order books.
- [**collections**](docs/collections.md) — grouping into maps, containers that allocate only when something is added, vectors that give memory back after a spike, running minimum and maximum, a stack-first vector and an async object pool.

### Background work — feature `with-tokio`

A service is assembled from long-living background components, and they share one lifecycle. Everything a component needs is passed to `new`, its handler is registered, and `start()` is called exactly once. The components marked *Startable* implement the `Startable` trait, so the application can collect them and start them in one loop. They report panics and timeouts through the application's `Logger`.

- [**timers**](docs/timers.md) — run code periodically: `MyTimer` every interval, `MyExactTimer` on wall-clock marks (`:00`, `:05`, …). A tick with more work than fits into its timeout continues in portions. *Startable.*
- [**events-loop**](docs/events-loop.md) — a single-consumer message loop: producers send from anywhere without locking, and one background task handles the events in order. *Startable.*
- [**background-executor**](docs/background-executor.md) — signal "there may be work" from any thread and let a background task do it. There is one task overall, or one per id (an account, an instrument), so that ids run in parallel while each one stays in order. *Startable.*
- [**queue-to-save**](docs/queue-to-save.md) — write-behind persistence: enqueue and return, while a background loop saves the items in chunks and retries a chunk that failed. A queue can keep only the latest state of each object, or turn it into a delete. *Startable.*
- [**idempotency**](docs/idempotency.md) — make a retried request execute at most once, identified by a request id or by a user id plus a request id.
- [**async-primitives**](docs/async-primitives.md) — a result that one task sets and another awaits, a gate many tasks wait on until initialization, and an in-memory byte pipe read through `AsyncRead`.
- [**app-lifecycle**](docs/app-lifecycle.md) — the `Logger` the components report to, the initialized and shutting-down states of the application (with SIGTERM / SIGINT handling), and `Startable`.

### Addresses and the rest

- [**remote-endpoint**](docs/remote-endpoint.md) — parse a connection string into scheme, host, port and path, with the default port of each scheme: HTTP and WebSocket URLs, unix sockets, SSH hosts and addresses reached through an SSH tunnel.
- [**misc**](docs/misc.md) — paths with `~` expansion, uuid v4, ids that sort by creation time, the `DataWrapper` derive and re-exports.

## For AI agents

[`docs/index_resource.md`](docs/index_resource.md) is the same map in compact form, with the topic list and the Startable types. development-mcp serves it as `resource://rust-extensions` and every topic as `resource://rust-extensions/{topic}`.

## License

MIT — see [LICENSE.md](LICENSE.md).
