# rust-extensions

A collection of useful Rust extensions and utilities for common programming tasks, including date/time handling, data structures, async utilities, and more.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rust-extensions = {git="https://github.com/rust-extensions/rust-extensions", tag="0.1.5"}
```

### Optional Features

- `with-tokio` - Enables async utilities and tokio-based features
- `base64` - Base64 encoding/decoding support
- `hex` - Hexadecimal encoding/decoding support
- `objects-pool` - Object pooling utilities
- `vec-maybe-stack` - Stack-allocated vector buffers

Example with features:

```toml
[dependencies]
rust-extensions = { version = "0.1.5", features = ["with-tokio", "base64"] }
```

## Overview

This crate provides various utility modules including:

- **Date/Time** - Comprehensive date and time handling with `DateTimeAsMicroseconds` and interval keys
- **Data Structures** - Sorted vectors, lazy collections, grouped data, and more
- **Async Utilities** - Event loops, task completion, tokio queues (requires `with-tokio` feature)
- **String Utilities** - String builders, short strings, and string manipulation
- **Iterators** - Array of bytes iterators, file iterators, and more
- **Binary Utilities** - Binary payload builders, hex encoding, base64 encoding
- **And more...**

## Table of Contents

- [Interval Key Module](#interval-key-module)
  - [Overview](#overview)
  - [Core Concepts](#core-concepts)
  - [Usage](#usage)
  - [DateTimeAsMicroseconds](#datetimeasmicroseconds)
  - [Implementation Details](#implementation-details)

---

# Interval Key Module

The `interval_key` module provides functionality to "cut" or round down `DateTime` values to specific time intervals. This is useful for grouping time-series data, creating time-based keys for databases, or aggregating data by time periods.

## Overview

The module offers two main approaches for working with time intervals:

1. **`IntervalKey<TOption>`** - A compile-time generic type that provides type safety and zero-cost abstractions. The interval type is determined at compile time using generics.

2. **`DateTimeInterval`** - A runtime enum that stores both the interval type and value, allowing for dynamic interval selection.

## Core Concepts

### Interval Types

The module supports the following interval types:

- **Year** - Rounds down to the start of the year (format: `YYYY`, e.g., `2021`)
- **Month** - Rounds down to the start of the month (format: `YYYYMM`, e.g., `202103`)
- **Day** - Rounds down to the start of the day (format: `YYYYMMDD`, e.g., `20210305`)
- **Hour** - Rounds down to the start of the hour (format: `YYYYMMDDHH`, e.g., `2021030501`)
- **Minute** - Rounds down to the start of the minute (format: `YYYYMMDDHHMM`, e.g., `202103050112`)
- **Minute5** - Rounds down to the nearest 5-minute interval (format: `YYYYMMDDHHMM`, e.g., `202103050110`)

### Key Format

Each interval type uses a compact integer representation:
- All values are stored as `i64`
- The format is hierarchical: year, month, day, hour, minute are concatenated
- For example, `2021-03-05T01:12:32` becomes:
  - Year: `2021`
  - Month: `202103`
  - Day: `20210305`
  - Hour: `2021030501`
  - Minute: `202103050112`
  - Minute5: `202103050110` (rounded to 5-minute boundary)

## Usage

### Using `IntervalKey<TOption>` (Compile-Time Type Safety)

`IntervalKey` uses Rust generics to ensure type safety at compile time. The interval type is encoded in the type system, preventing mixing different interval types.

```rust
use rust_extensions::date_time::*;
use rust_extensions::date_time::interval_key::*;

// Create an interval key from a DateTime
let dt = DateTimeAsMicroseconds::from_str("2021-03-05T01:12:32.000000Z").unwrap();

// Create different interval keys
let year_key: IntervalKey<YearKey> = dt.into();
let month_key: IntervalKey<MonthKey> = dt.into();
let day_key: IntervalKey<DayKey> = dt.into();
let hour_key: IntervalKey<HourKey> = dt.into();
let minute_key: IntervalKey<MinuteKey> = dt.into();
let minute5_key: IntervalKey<Minute5Key> = dt.into();

// Access the integer value
assert_eq!(year_key.to_i64(), 2021);
assert_eq!(month_key.to_i64(), 202103);
assert_eq!(day_key.to_i64(), 20210305);
assert_eq!(hour_key.to_i64(), 2021030501);
assert_eq!(minute_key.to_i64(), 202103050112);
assert_eq!(minute5_key.to_i64(), 202103050110);

// Convert back to DateTime
let dt_result: DateTimeAsMicroseconds = year_key.try_into().unwrap();
// Result: "2021-01-01T00:00:00"
```

### Using `DateTimeInterval` (Runtime Flexibility)

`DateTimeInterval` stores the interval type as a value, allowing for dynamic interval selection at runtime.

```rust
use rust_extensions::date_time::*;
use rust_extensions::date_time::interval_key::*;

let dt = DateTimeAsMicroseconds::from_str("2021-03-05T01:12:32.000000Z").unwrap();

// Create interval based on runtime condition
let interval = if some_condition {
    DateTimeInterval::from_dt_to_hour(dt)
} else {
    DateTimeInterval::from_dt_to_day(dt)
};

// Access the value
let value = interval.to_i64();

// Convert back to DateTime
let dt_result = interval.to_date_time().unwrap();
```

### Converting Between Interval Types

You can convert between compatible interval types:

```rust
// Convert from DateTime to MinuteKey
let minute_key: IntervalKey<MinuteKey> = dt.into();

// Convert to Minute5Key
let minute5_key: IntervalKey<Minute5Key> = minute_key.try_into().unwrap();

// Convert back to MinuteKey
let minute_key_again: IntervalKey<MinuteKey> = minute5_key.try_into().unwrap();
```

### Arithmetic Operations

You can add or subtract durations from interval keys:

```rust
use std::time::Duration;

let hour_key: IntervalKey<HourKey> = dt.into();

// Add 2 hours
let next_hour = hour_key.add(Duration::from_secs(2 * 3600));

// Subtract 1 hour
let prev_hour = hour_key.sub(Duration::from_secs(3600));
```

### Converting to `DateTimeInterval`

You can convert a compile-time `IntervalKey` to a runtime `DateTimeInterval`:

```rust
let hour_key: IntervalKey<HourKey> = dt.into();
let interval = hour_key.to_dt_interval();
// interval is DateTimeInterval::Hour(value)
```

## Available Types

### Interval Key Types (Compile-Time)

- `IntervalKey<YearKey>` - Year-level intervals
- `IntervalKey<MonthKey>` - Month-level intervals
- `IntervalKey<DayKey>` - Day-level intervals
- `IntervalKey<HourKey>` - Hour-level intervals
- `IntervalKey<MinuteKey>` - Minute-level intervals
- `IntervalKey<Minute5Key>` - 5-minute intervals

### DateTimeInterval Variants (Runtime)

- `DateTimeInterval::Year(i64)`
- `DateTimeInterval::Month(i64)`
- `DateTimeInterval::Day(i64)`
- `DateTimeInterval::Hour(i64)`
- `DateTimeInterval::Minute(i64)`
- `DateTimeInterval::Min5(i64)`

## When to Use Which?

### Use `IntervalKey<TOption>` when:
- The interval type is known at compile time
- You want type safety to prevent mixing different interval types
- You want zero-cost abstractions (no runtime overhead for type checking)
- You're working with collections of the same interval type

### Use `DateTimeInterval` when:
- The interval type needs to be determined at runtime
- You need to store different interval types in the same collection
- You're working with dynamic or user-configurable interval types
- You need to serialize/deserialize interval information

## Examples

### Grouping Data by Time Intervals

```rust
use std::collections::HashMap;
use rust_extensions::date_time::*;
use rust_extensions::date_time::interval_key::*;

// Group events by hour
let mut events_by_hour: HashMap<IntervalKey<HourKey>, Vec<Event>> = HashMap::new();

for event in events {
    let hour_key: IntervalKey<HourKey> = event.timestamp.into();
    events_by_hour.entry(hour_key).or_insert_with(Vec::new).push(event);
}
```

### Dynamic Interval Selection

```rust
fn aggregate_by_interval(
    dt: DateTimeAsMicroseconds,
    interval_type: &str
) -> DateTimeInterval {
    match interval_type {
        "hour" => DateTimeInterval::from_dt_to_hour(dt),
        "day" => DateTimeInterval::from_dt_to_day(dt),
        "month" => DateTimeInterval::from_dt_to_month(dt),
        _ => DateTimeInterval::from_dt_to_minute(dt),
    }
}
```

## DateTimeAsMicroseconds

`DateTimeAsMicroseconds` is the base datetime type used throughout the `interval_key` module. It's a lightweight, efficient representation of a point in time.

### Design

`DateTimeAsMicroseconds` is a **single-field structure** that stores a Unix timestamp in **UTC+0** (Coordinated Universal Time) as microseconds since the Unix epoch (January 1, 1970, 00:00:00 UTC).

```rust
pub struct DateTimeAsMicroseconds {
    pub unix_microseconds: i64,
}
```

### Key Characteristics

- **Lightweight**: Contains only a single `i64` field (8 bytes)
- **Copyable**: Implements `Copy` and `Clone` traits, making it cheap to pass around
- **UTC+0**: Always represents time in UTC, eliminating timezone confusion
- **Microsecond Precision**: Stores time with microsecond precision
- **Serializable**: Supports `Serialize` and `Deserialize` via serde (with transparent serialization)

### Creating DateTimeAsMicroseconds

```rust
use rust_extensions::date_time::*;

// From a string (various formats supported)
let dt1 = DateTimeAsMicroseconds::from_str("2021-03-05T01:12:32.000000Z").unwrap();
let dt2 = DateTimeAsMicroseconds::from_str("2021-03-05").unwrap(); // Date only
let dt3 = DateTimeAsMicroseconds::parse_iso_string("2021-03-05T01:12:32.000Z").unwrap();

// From Unix timestamp (automatically detects seconds, milliseconds, or microseconds)
let dt4: DateTimeAsMicroseconds = 1679059876i64.into();        // seconds
let dt5: DateTimeAsMicroseconds = 1679059876123i64.into();     // milliseconds
let dt6: DateTimeAsMicroseconds = 1679059876123456i64.into();  // microseconds

// Direct creation
let dt7 = DateTimeAsMicroseconds::new(1679059876123456i64);

// Current time
let now = DateTimeAsMicroseconds::now();

// From components
let dt8 = DateTimeAsMicroseconds::create(2021, 3, 5, 1, 12, 32, 0);
```

### Common Operations

```rust
use std::time::Duration;

// Arithmetic operations
let dt = DateTimeAsMicroseconds::from_str("2021-03-05T01:12:32.000000Z").unwrap();

// Add/subtract durations
let later = dt.add(Duration::from_secs(3600));  // Add 1 hour
let earlier = dt.sub(Duration::from_secs(3600)); // Subtract 1 hour

// Add time units (mutating)
let mut dt = dt;
dt.add_seconds(60);
dt.add_minutes(5);
dt.add_hours(2);
dt.add_days(1);

// Comparisons
if dt1.is_later_than(dt2) {
    // dt1 is after dt2
}

if dt1.is_earlier_than(dt2) {
    // dt1 is before dt2
}

// Calculate duration between two times
let duration = dt1.duration_since(dt2);
let seconds_diff = dt1.seconds_before(dt2);

// Formatting
let rfc3339 = dt.to_rfc3339();        // "2021-03-05T01:12:32.000000Z"
let rfc2822 = dt.to_rfc2822();        // RFC 2822 format
let rfc5322 = dt.to_rfc5322();        // RFC 5322 format
let rfc7231 = dt.to_rfc7231();        // RFC 7231 format
let compact = dt.to_compact_date_time_string();

// Convert to chrono DateTime<Utc>
let chrono_dt = dt.to_chrono_utc();
```

### Why Use DateTimeAsMicroseconds?

1. **Performance**: Single-field structure means minimal memory overhead and fast comparisons
2. **Simplicity**: No timezone handling complexity - everything is in UTC
3. **Precision**: Microsecond precision is sufficient for most use cases
4. **Interoperability**: Easy conversion to/from Unix timestamps
5. **Type Safety**: Distinct type prevents mixing with raw integers
6. **Copy Semantics**: `Copy` trait means no heap allocation or reference management needed

### Integration with Interval Keys

`DateTimeAsMicroseconds` is the primary input type for creating interval keys:

```rust
let dt = DateTimeAsMicroseconds::from_str("2021-03-05T01:12:32.000000Z").unwrap();

// All interval key types can be created from DateTimeAsMicroseconds
let hour_key: IntervalKey<HourKey> = dt.into();
let day_key: IntervalKey<DayKey> = dt.into();
// ... etc
```

And interval keys can be converted back to `DateTimeAsMicroseconds`:

```rust
let hour_key: IntervalKey<HourKey> = dt.into();
let dt_result: DateTimeAsMicroseconds = hour_key.try_into().unwrap();
// Result represents the start of that hour interval
```

## Implementation Details

- All interval keys are stored as `i64` values for efficient storage and comparison
- The module uses a phantom type pattern to ensure type safety without runtime overhead
- Conversion between `DateTimeAsMicroseconds` and interval keys is done through utility functions in the `utils` module
- The `IntervalKeyOption` trait defines the behavior for each interval type

## See Also

- `DateTimeAsMicroseconds` - The base datetime type used by this module (see detailed section above)
- `DateTimeStruct` - Used internally for date/time calculations and conversions

---

## License

This project is licensed under the MIT License - see the [LICENSE.md](LICENSE.md) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Version

Current version: **0.1.5**

