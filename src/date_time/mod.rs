mod as_microseconds;
mod as_microseconds_atomic;
pub mod compact_date_time;
mod date_time_duration;
mod date_time_struct;
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

static MONTHS: [&'static str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

static WEEKS: [&'static str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
