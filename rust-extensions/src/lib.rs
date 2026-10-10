#[cfg(feature = "with-tokio")]
mod application_states;
mod binary_payload_builder;
pub mod date_time;
pub mod duration_utils;
#[cfg(all(feature = "with-tokio", not(target_arch = "wasm32")))]
pub mod events_loop;
#[cfg(all(feature = "with-tokio", not(target_arch = "wasm32")))]
pub mod background_executor;
#[cfg(all(feature = "with-tokio", not(target_arch = "wasm32")))]
pub mod background_executor_with_multi_threads;
pub mod lazy;
pub mod linq;
mod logger;
#[cfg(all(feature = "with-tokio", not(target_arch = "wasm32")))]
mod my_timer;
#[cfg(all(feature = "with-tokio", not(target_arch = "wasm32")))]
mod exact_timer;
mod short_string;
mod slice_or_vec;
pub use short_string::*;
// pooling native resources - nothing to pool in a browser, and it needs `tokio`
#[cfg(all(feature = "objects-pool", not(target_arch = "wasm32")))]
pub mod objects_pool;

pub mod slice_of_u8_utils;
mod stop_watch;
mod str_or_string;
mod secure_string_builder;
mod string_builder;
#[cfg(feature = "with-tokio")]
mod is_initialized;
#[cfg(feature = "with-tokio")]
pub use is_initialized::*;
#[cfg(feature = "with-tokio")]
pub mod idempotency;
#[cfg(feature = "with-tokio")]
mod task_completion;
#[cfg(feature = "with-tokio")]
pub mod tokio_queue;

#[cfg(feature = "with-tokio")]
pub use application_states::*;
pub use stop_watch::StopWatch;
pub use secure_string_builder::SecureStringBuilder;
pub use string_builder::StringBuilder;
#[cfg(feature = "with-tokio")]
pub use task_completion::{TaskCompletion, TaskCompletionAwaiter, TaskCompletionError};
pub mod grouped_data;

pub use binary_payload_builder::*;
pub use logger::*;
#[cfg(all(feature = "with-tokio", not(target_arch = "wasm32")))]
pub use my_timer::{MyTimer, MyTimerTick, RepeatTimerIteration};
#[cfg(all(feature = "with-tokio", not(target_arch = "wasm32")))]
pub use exact_timer::{ExactTimerInterval, MyExactTimer};
pub use slice_or_vec::*;
pub use str_or_string::*;
pub mod auto_shrink;
#[cfg(feature = "base64")]
pub mod base64;
pub mod file_utils;
#[cfg(feature = "hex")]
pub mod hex;
pub mod sorted_vec;
pub mod str_utils;
#[cfg(feature = "vec-maybe-stack")]
pub mod vec_maybe_stack;
pub mod array_of_bytes_iterator;
mod maybe_short_string;
pub use maybe_short_string::*;

pub extern crate chrono;
mod min_value;
pub use min_value::*;
mod max_value;
pub use max_value::*;
pub mod remote_endpoint;

mod sorted_ver_with_2_keys;
pub use sorted_ver_with_2_keys::*;

mod atomic_stop_watch;
pub use atomic_stop_watch::*;
mod atomic_duration;
pub use atomic_duration::*;
mod min_key_value;
pub use min_key_value::*;
pub mod binary_search;
#[cfg(any(feature = "rnd", target_arch = "wasm32"))]
pub mod uuid;
#[cfg(any(feature = "rnd", target_arch = "wasm32"))]
mod sortable_id;
#[cfg(any(feature = "rnd", target_arch = "wasm32"))]
pub use sortable_id::*;
mod uint32_variable_size;
pub use uint32_variable_size::*;

#[cfg(all(feature = "with-tokio", not(target_arch = "wasm32")))]
mod queue_to_save;
#[cfg(all(feature = "with-tokio", not(target_arch = "wasm32")))]
pub use queue_to_save::*;
mod as_str;
pub use as_str::*;
mod async_bytes_stream;
pub use async_bytes_stream::*;
mod buffered_reader;
pub use buffered_reader::*;
mod double_buffer;
pub use double_buffer::*;
// needs `tokio::fs`, which does not exist on wasm
#[cfg(all(feature = "with-tokio", not(target_arch = "wasm32")))]
mod file_stream_reader;
#[cfg(all(feature = "with-tokio", not(target_arch = "wasm32")))]
pub use file_stream_reader::*;

pub extern crate macros;

#[cfg(all(feature = "with-tokio", not(target_arch = "wasm32")))]
mod queue_to_save_with_id;
#[cfg(all(feature = "with-tokio", not(target_arch = "wasm32")))]
pub use queue_to_save_with_id::*;

mod sized_chunks;
pub use sized_chunks::*;

mod vec_uninit;

#[cfg(all(feature = "with-tokio", not(target_arch = "wasm32")))]
mod queue_to_save_or_delete_with_id;
#[cfg(all(feature = "with-tokio", not(target_arch = "wasm32")))]
pub use queue_to_save_or_delete_with_id::*;

#[cfg(all(feature = "with-tokio", not(target_arch = "wasm32")))]
mod startable;
#[cfg(all(feature = "with-tokio", not(target_arch = "wasm32")))]
pub use startable::*;

/// Every `rust` block of `docs/*.md` is compiled - and the synchronous ones run - by
/// `cargo test --doc --all-features`, so the docs can not drift away from the API.
#[cfg(doctest)]
mod docs {
    #[doc = include_str!("../../docs/date-time.md")]
    struct DateTime;
    #[doc = include_str!("../../docs/interval-keys.md")]
    struct IntervalKeys;
    #[doc = include_str!("../../docs/durations.md")]
    struct Durations;
    #[doc = include_str!("../../docs/strings.md")]
    struct Strings;
    #[doc = include_str!("../../docs/secure-string-builder.md")]
    struct SecureStringBuilder;
    #[cfg(all(feature = "hex", feature = "base64", feature = "with-tokio"))]
    #[doc = include_str!("../../docs/binary.md")]
    struct Binary;
    #[doc = include_str!("../../docs/sorted-vec.md")]
    struct SortedVec;
    #[cfg(all(feature = "vec-maybe-stack", feature = "objects-pool"))]
    #[doc = include_str!("../../docs/collections.md")]
    struct Collections;
    #[doc = include_str!("../../docs/sized-chunks.md")]
    struct SizedChunks;
    #[cfg(feature = "with-tokio")]
    #[doc = include_str!("../../docs/timers.md")]
    struct Timers;
    #[cfg(feature = "with-tokio")]
    #[doc = include_str!("../../docs/events-loop.md")]
    struct EventsLoop;
    #[cfg(feature = "with-tokio")]
    #[doc = include_str!("../../docs/background-executor.md")]
    struct BackgroundExecutor;
    #[cfg(feature = "with-tokio")]
    #[doc = include_str!("../../docs/queue-to-save.md")]
    struct QueueToSave;
    #[cfg(feature = "with-tokio")]
    #[doc = include_str!("../../docs/idempotency.md")]
    struct Idempotency;
    #[cfg(feature = "with-tokio")]
    #[doc = include_str!("../../docs/async-primitives.md")]
    struct AsyncPrimitives;
    #[cfg(feature = "with-tokio")]
    #[doc = include_str!("../../docs/app-lifecycle.md")]
    struct AppLifecycle;
    #[doc = include_str!("../../docs/remote-endpoint.md")]
    struct RemoteEndpoint;
    #[cfg(feature = "rnd")]
    #[doc = include_str!("../../docs/misc.md")]
    struct Misc;
}
