mod application_states;
pub mod date_time;
pub mod duration_utils;
pub mod events_loop;
pub mod lazy;
pub mod linq;
mod logger;
mod my_timer;
#[cfg(feature = "objects-pool")]
pub mod objects_pool;

pub mod slice_of_u8_utils;
mod stop_watch;
mod str_or_string;
mod string_builder;
mod task_completion;

pub use stop_watch::StopWatch;
pub use string_builder::StringBuilder;
pub use task_completion::{TaskCompletion, TaskCompletionAwaiter, TaskCompletionError};

pub use application_states::*;
pub mod grouped_data;

pub use logger::Logger;
pub use my_timer::{MyTimer, MyTimerTick};
pub use str_or_string::*;
