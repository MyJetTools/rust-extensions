# async-primitives

Feature `with-tokio`. Building blocks for coordinating tasks.

## TaskCompletion — a one-shot result that someone else sets

A oneshot channel that carries `Result<Ok, Err>` and knows what to say when the setting side is dropped. One side keeps the `TaskCompletion` and sets the result. The other side takes the awaiter and waits.

```rust
use rust_extensions::{TaskCompletion, TaskCompletionAwaiter};

let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();

rt.block_on(async {
    let mut completion: TaskCompletion<u32, String> = TaskCompletion::new();
    let awaiter: TaskCompletionAwaiter<u32, String> = completion.get_awaiter(); // only once

    tokio::spawn(async move {
        completion.set_ok(42); // or set_error(..); set_* twice panics, try_set_* returns Err
    });

    assert_eq!(awaiter.get_result().await, Ok(42));

    // Dropped without a result: the awaiter gets the drop error, if one was given...
    let mut completion: TaskCompletion<u32, String> = TaskCompletion::new();
    let awaiter = completion.get_awaiter();
    completion.set_drop_error("cancelled".to_string());
    drop(completion);
    assert_eq!(awaiter.get_result().await, Err("cancelled".to_string()));

    // ...otherwise get_result() panics with "Task is dropped".
});
```

`set_panic(message)` makes the awaiter panic with that message. `TaskCompletionAwaiter::create_completed(result)` is an awaiter that is ready from the start.

## IsInitialized — a gate many tasks wait on

Any number of tasks `wait_until_initialized().await` until somebody calls `initialized().await` once. After that every wait returns at once through an atomic flag. It lives in an `AppCtx` as a plain field.

```rust
use std::time::Duration;
use rust_extensions::IsInitialized;

let rt = tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap();

rt.block_on(async {
    let gate = std::sync::Arc::new(IsInitialized::new());

    let waiter = tokio::spawn({
        let gate = gate.clone();
        async move { gate.wait_until_initialized().await }
    });

    assert!(!gate.is_initialized());
    gate.initialized().await; // releases every waiter; calling it again is a no-op
    waiter.await.unwrap();

    gate.panic_if_not_initialized(); // panics with "Not Initialized" before initialized()
    gate.wait_some_time_and_panic(Duration::from_secs(1)).await; // or panics after the timeout
});
```

A waiter whose future was dropped before initialization is skipped, so nothing panics. There are no lost wake-ups.

## TokioQueue — an in-memory byte pipe read as AsyncRead

Producers `enqueue(&[u8])` through a publisher, without awaiting. The queue itself is the reading side: anything that takes `tokio::io::AsyncRead` can drain it.

```rust
use rust_extensions::tokio_queue::TokioQueue;
use tokio::io::AsyncReadExt;

let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();

rt.block_on(async {
    let mut queue = TokioQueue::new();
    let publisher = queue.get_publisher(); // Arc - clone it into the producers

    publisher.enqueue(b"hello ");
    publisher.enqueue(b"world");

    let mut buf = [0u8; 11];
    queue.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"hello world");
});
```

- **Unbounded.** Nothing pushes back on producers.
- **No end of stream.** A read on an empty queue waits for the next `enqueue`.
