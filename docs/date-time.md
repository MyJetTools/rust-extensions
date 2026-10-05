# date-time

`rust_extensions::date_time` — a UTC instant as `i64` microseconds since the Unix epoch, and what is built around it.

## DateTimeAsMicroseconds

A single public field, `unix_microseconds: i64`. `Copy`, `Ord`, always UTC.

```rust
use rust_extensions::date_time::DateTimeAsMicroseconds;

let now = DateTimeAsMicroseconds::now();
let dt = DateTimeAsMicroseconds::new(1_619_371_803_000_000);

// y, m, d, h, min, s, microseconds
assert_eq!(DateTimeAsMicroseconds::create(2021, 4, 25, 17, 30, 3, 0), dt);

assert_eq!(DateTimeAsMicroseconds::parse_iso_string("2021-04-25T17:30:03Z"), Some(dt));

// from_str takes RFC 3339, a date, a date-time without seconds, or a number
assert_eq!(DateTimeAsMicroseconds::from_str("2021-04-25T17:30:03.000000Z"), Some(dt));
assert_eq!(DateTimeAsMicroseconds::from_str("1619371803"), Some(dt));
assert!(DateTimeAsMicroseconds::from_str("2021-04-25").is_some());

// A number carries no unit: From<i64> sniffs seconds / millis / micros / nanos by magnitude.
let from_secs: DateTimeAsMicroseconds = 1_619_371_803i64.into();
let from_millis: DateTimeAsMicroseconds = 1_619_371_803_000i64.into();
let from_nanos: DateTimeAsMicroseconds = 1_619_371_803_000_000_000i64.into();
assert_eq!(from_secs, dt);
assert_eq!(from_millis, dt);
assert_eq!(from_nanos, dt);
assert_eq!(DateTimeAsMicroseconds::from_nanos(1_619_371_803_000_000_000), dt);

assert!(now.is_later_than(dt));
```

Arithmetic and comparison:

```rust
use std::time::Duration;
use rust_extensions::date_time::{DateTimeAsMicroseconds, DateTimeDuration};

let start = DateTimeAsMicroseconds::create(2021, 4, 25, 17, 30, 3, 0);

let later = start.add(Duration::from_secs(90));
assert_eq!(later.sub(Duration::from_secs(90)), start);

let mut moved = start;
moved.add_days(1);
moved.add_hours(-1);
moved.add_minutes(30);
moved.add_seconds(-30);

assert_eq!(later.seconds_before(start), 90);
assert!(later.is_later_than(start) && start.is_earlier_than(later));
assert!(start < later);

// `-` and duration_since give a signed DateTimeDuration
match later - start {
    DateTimeDuration::Positive(duration) => assert_eq!(duration, Duration::from_secs(90)),
    _ => unreachable!(),
}
assert_eq!((start - later).get_full_seconds(), -90);
assert_eq!((start - later).as_positive_or_zero(), Duration::ZERO);
assert_eq!(later.duration_since(start).to_string(), "+00:01:30");
```

Renderings — `Display` is the **number**, `Debug` and serde are strings:

```rust
use rust_extensions::date_time::{DateTimeAsMicroseconds, DateTimeStruct};

let dt = DateTimeAsMicroseconds::create(2021, 4, 25, 17, 30, 3, 0);

assert_eq!(dt.to_string(), "1619371803000000");
assert_eq!(format!("{:?}", dt), "'2021-04-25T17:30:03+00:00'");
assert_eq!(dt.to_rfc3339(), "2021-04-25T17:30:03+00:00");
assert_eq!(dt.to_rfc3339_utc(), "2021-04-25T17:30:03.000000Z"); // fixed width, sorts as text
assert_eq!(dt.to_rfc7231(), "Sun, 25 Apr 2021 17:30:03 GMT"); // HTTP headers
assert_eq!(dt.to_rfc2822(), "Sun, 25 Apr 2021 17:30:03 +0000");
assert_eq!(dt.to_rfc5322(), "Apr 25 17:30:03 2021 GMT"); // despite the name: the X.509 / OpenSSL form
assert_eq!(dt.to_compact_date_time_string(), "20210425173003");
assert_eq!(dt.to_chrono_utc().timestamp(), 1_619_371_803);

let parts: DateTimeStruct = dt.into();
assert_eq!((parts.year, parts.month, parts.day), (2021, 4, 25));
assert_eq!((parts.time.hour, parts.time.min, parts.time.sec), (17, 30, 3));
```

The parsers ignore a non-zero offset: `2024-01-02T03:04:05+03:00` reads as `03:04:05` UTC. `Z` and `+00:00` are fine.

### Serde

Asymmetric on purpose — keep it that way.

- **Writes** RFC 3339 in UTC, `Z`, 6 fraction digits: `"2021-04-25T17:30:03.000000Z"`. Never a number.
- **Reads** that string, an RFC 3339 with `+00:00`, and a unix timestamp as a number or as digits in a string. A number goes through `From<i64>`, so its unit is sniffed.

Data stored by older versions as `1619371803000000` stays readable. Reading uses `deserialize_any`, so it needs a self-describing format such as JSON.

```rust
use rust_extensions::date_time::DateTimeAsMicroseconds;

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct Deal {
    created: DateTimeAsMicroseconds,
}

let deal = Deal { created: DateTimeAsMicroseconds::create(2021, 4, 25, 17, 30, 3, 0) };

let json = serde_json::to_string(&deal).unwrap();
assert_eq!(json, r#"{"created":"2021-04-25T17:30:03.000000Z"}"#);

let old: Deal = serde_json::from_str(r#"{"created":1619371803000000}"#).unwrap();
assert_eq!(old, deal);
```

A hand-written deserializer passes the raw JSON token, quotes included, to `from_json_value_str`, which accepts the same spellings serde does:

```rust
use rust_extensions::date_time::DateTimeAsMicroseconds;

let dt = DateTimeAsMicroseconds::from_json_value_str("\"2021-04-25T17:30:03.000000Z\"");
assert_eq!(dt, DateTimeAsMicroseconds::from_json_value_str("1619371803000000"));
assert_eq!(dt, DateTimeAsMicroseconds::from_json_value_str("\"1619371803\""));
assert_eq!(DateTimeAsMicroseconds::from_json_value_str("null"), None);
```

## AtomicDateTimeAsMicroseconds

The same instant behind an atomic — share it and `update` through `&self`.

```rust
use rust_extensions::date_time::{AtomicDateTimeAsMicroseconds, DateTimeAsMicroseconds};

let last_seen = AtomicDateTimeAsMicroseconds::now();

let dt = DateTimeAsMicroseconds::create(2021, 4, 25, 17, 30, 3, 0);
last_seen.update(dt);

assert_eq!(last_seen.as_date_time(), dt);
assert_eq!(last_seen.get_unix_microseconds(), dt.unix_microseconds);
```

## DateTimeAsMicrosecondsWithTimeZone

A UTC instant plus the fixed offset to show it in. `TimeZone` is that offset in minutes: `UTC+1` is `60`, `UTC+5:45` is `345`.

`from_server_and_local_time` derives the offset as `local - server`, rounded to 15 minutes, so a noisy client clock still lands on a real zone.

```rust
use rust_extensions::date_time::*;

let server = DateTimeAsMicroseconds::create(2021, 4, 25, 17, 30, 3, 0);

// The client reports its wall clock for the same moment as 18:28.
let mut local = server;
local.add_minutes(58);

let dt = DateTimeAsMicrosecondsWithTimeZone::from_server_and_local_time(server, local);

assert_eq!(dt.time_zone.offset_in_minutes(), 60);
assert_eq!(dt.to_compact_string(), "2021-04-25 18:30:03");
assert_eq!(dt.to_rfc3339(), "2021-04-25T18:30:03.000000+01:00");
assert_eq!(dt.to_local_date_time_struct().time.hour, 18);

let explicit = DateTimeAsMicrosecondsWithTimeZone::new(server, TimeZone::from_minutes(-300));
assert_eq!(format!("{:?}", explicit.time_zone), "-05:00");
```

Serde writes and reads exactly `to_rfc3339()`. A bare number, or a string without an offset, is refused.

## Client time

A time typed in by a user is on the user's clock. `client_input_time_to_server_time` shifts it by the client/server difference, rounded to 30 minutes:

```rust
use rust_extensions::date_time::DateTimeAsMicroseconds;

let server_now = DateTimeAsMicroseconds::create(2021, 4, 25, 17, 0, 0, 0);
let client_now = DateTimeAsMicroseconds::create(2021, 4, 25, 20, 1, 0, 0); // UTC+3, a minute off

let client_input = DateTimeAsMicroseconds::create(2021, 4, 25, 21, 0, 0, 0);

let on_server =
    DateTimeAsMicroseconds::client_input_time_to_server_time(client_input, client_now, server_now);

assert_eq!(on_server, DateTimeAsMicroseconds::create(2021, 4, 25, 18, 0, 0, 0));
assert_eq!(client_now.get_client_server_time_difference(server_now).difference_in_hours(), 3);
```
