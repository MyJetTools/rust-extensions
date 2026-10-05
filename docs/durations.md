# durations

Parse and print `std::time::Duration`, measure elapsed time, keep a duration that can be changed through `&self`.

## Parse and print — `duration_utils`

`parse_duration` reads the forms people put into settings:

- `150ms`
- `mm:ss`
- `hh:mm:ss`
- `<days>d hh:mm:ss`

A bare number is refused, so `"30"` is an error and `"00:30"` is not.

`duration_to_string` prints below a minute as `Debug` does, and from a minute on as `hh:mm:ss` with an optional `<days>d:` prefix.

```rust
use std::time::Duration;
use rust_extensions::duration_utils::{duration_to_string, parse_duration, DurationExtensions};

assert_eq!(parse_duration("150ms").unwrap(), Duration::from_millis(150));
assert_eq!(parse_duration("01:30").unwrap(), Duration::from_secs(90));
assert_eq!(parse_duration("02:00:05").unwrap(), Duration::from_secs(2 * 3600 + 5));
assert_eq!(parse_duration("1d 00:00:01").unwrap(), Duration::from_secs(86_400 + 1));
assert!(parse_duration("30").is_err());

assert_eq!(duration_to_string(Duration::from_millis(150)), "150ms");
assert_eq!(duration_to_string(Duration::from_secs(61)), "00:01:01");
assert_eq!(duration_to_string(Duration::from_secs(15 * 86_400 + 3661)), "15d:01:01:01");

// The same as a trait on Duration
assert_eq!(Duration::from_str("00:10").unwrap(), Duration::from_secs(10));
assert_eq!(Duration::from_secs(61).format_to_string(), "00:01:01");
```

## StopWatch

Wall-clock time since `new()` or `reset()`. `start()` and `pause()` are deprecated no-ops.

```rust
use rust_extensions::StopWatch;

let mut sw = StopWatch::new();
// ... work ...
let elapsed = sw.duration();
let printed = sw.duration_as_string(); // "35ms", "00:01:05"
let started_at = sw.get_start_time(); // DateTimeAsMicroseconds

sw.reset();
assert!(sw.duration() <= elapsed + std::time::Duration::from_secs(1));
```

## AtomicStopWatch

Shared through `&self`. `duration()` is the time between `reset_and_start()` and the last `pause()`. It is zero until the first `pause()`.

```rust
use rust_extensions::{date_time::DateTimeDuration, AtomicStopWatch};

let sw = AtomicStopWatch::new();

sw.reset_and_start();
assert!(matches!(sw.duration(), DateTimeDuration::Zero));

// ... work ...
sw.pause();
let elapsed = sw.duration().as_positive_or_zero();
```

## AtomicDuration

A `Duration` stored as microseconds in an `AtomicU64` — a setting that can be changed at runtime without a lock.

```rust
use std::time::Duration;
use rust_extensions::AtomicDuration;

let timeout = AtomicDuration::from_secs(5);
assert_eq!(timeout.to_duration(), Duration::from_secs(5));

timeout.update(Duration::from_millis(1500));
assert_eq!(timeout.get_micros(), 1_500_000);

let as_duration: Duration = (&timeout).into();
assert_eq!(as_duration, Duration::from_millis(1500));

let from_duration: AtomicDuration = Duration::from_millis(10).into();
assert_eq!(AtomicDuration::from_millis(10).get_micros(), from_duration.get_micros());
assert_eq!(AtomicDuration::from_micros(7).get_micros(), 7);
```
