#[cfg(feature = "with-tokio")]
mod application_states;
mod binary_payload_builder;
pub mod date_time;
pub mod duration_utils;
#[cfg(feature = "with-tokio")]
pub mod events_loop;
pub mod lazy;
pub mod linq;
mod logger;
#[cfg(feature = "with-tokio")]
mod my_timer;
mod short_string;
mod slice_or_vec;
pub use short_string::*;
#[cfg(feature = "objects-pool")]
pub mod objects_pool;

pub mod slice_of_u8_utils;
mod stop_watch;
mod str_or_string;
mod string_builder;
#[cfg(feature = "with-tokio")]
mod task_completion;
#[cfg(feature = "with-tokio")]
pub mod tokio_queue;

#[cfg(feature = "with-tokio")]
pub use application_states::*;
pub use stop_watch::StopWatch;
pub use string_builder::StringBuilder;
#[cfg(feature = "with-tokio")]
pub use task_completion::{TaskCompletion, TaskCompletionAwaiter, TaskCompletionError};
pub mod grouped_data;

pub use binary_payload_builder::*;
pub use logger::*;
#[cfg(feature = "with-tokio")]
pub use my_timer::{MyTimer, MyTimerTick};
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
mod unsafe_value;
#[cfg(feature = "vec-maybe-stack")]
pub mod vec_maybe_stack;
pub use unsafe_value::*;
pub mod array_of_bytes_iterator;
mod maybe_short_string;
pub use maybe_short_string::*;
#[cfg(feature = "placeholders")]
pub mod placeholders;

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
