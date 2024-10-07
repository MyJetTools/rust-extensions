mod as_microseconds;
mod as_microseconds_atomic;
pub mod compact_date_time;
mod date_time_duration;
mod date_time_struct;
mod interval_key;
pub mod rfc_3339;
pub mod rfc_5322;
mod time_difference;
mod time_struct;
mod utils;

pub use as_microseconds::DateTimeAsMicroseconds;
pub use as_microseconds_atomic::AtomicDateTimeAsMicroseconds;

pub use date_time_duration::DateTimeDuration;
pub use date_time_struct::*;
pub use time_difference::*;
pub use time_struct::*;
pub use utils::*;
pub mod rfc_7231;
pub use interval_key::*;

static MONTHS: [&'static str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

static WEEKS: [&'static str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

pub const MICRO_SECONDS_IN_ONE_SECOND: i64 = 1_000_000;
pub const MICRO_SECONDS_IN_ONE_MINUTE: i64 = 60 * MICRO_SECONDS_IN_ONE_SECOND;
pub const MICRO_SECONDS_IN_ONE_HOUR: i64 = 60 * MICRO_SECONDS_IN_ONE_MINUTE;
pub const MICRO_SECONDS_IN_ONE_DAY: i64 = 24 * MICRO_SECONDS_IN_ONE_HOUR;
