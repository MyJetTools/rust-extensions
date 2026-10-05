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

// A number carries no unit: From<i64> sniffs it by magnitude. Below 4_733_514_061 it is
// seconds, below that x1000 millis, below x1_000_000 micros, else nanos - each range
// reaches 2120-01-01. For a unit known up front: DateTimeAsMicroseconds::new(millis * 1000).
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
use rust_extensions::date_time::{
    DateTimeAsMicroseconds, DateTimeAsMicrosecondsWithTimeZone, DateTimeStruct, TimeZone,
};

let dt = DateTimeAsMicroseconds::create(2021, 4, 25, 17, 30, 3, 0);

// The readable `YYYY-MM-DD HH:MM:SS` goes through the zoned type, here with UTC
let readable = DateTimeAsMicrosecondsWithTimeZone::new(dt, TimeZone::utc()).to_compact_string();
assert_eq!(readable, "2021-04-25 17:30:03");

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

## DateTimeStruct — calendar fields

The UTC instant taken apart: `year`, `month`, `day`, `time` (`hour`, `min`, `sec`, `micros`) and `dow: Option<Weekday>`. It is the way to work with calendar fields — step a month, compare dates, get the day of the week.

```rust
use rust_extensions::chrono::Weekday;
use rust_extensions::date_time::{DateTimeAsMicroseconds, DateTimeStruct};

let dt = DateTimeAsMicroseconds::create(2021, 4, 25, 17, 30, 3, 0);
let mut parts: DateTimeStruct = dt.into();

parts.inc_month(); // the same day of the next month; dec_month() goes back
assert_eq!((parts.year, parts.month, parts.day), (2021, 5, 25));
assert_eq!(parts.get_day_of_week(), Weekday::Tue);
assert_eq!(parts.get_day_of_week_as_str(), "Tue");

let back: DateTimeAsMicroseconds = parts.clone().try_into().unwrap();
assert_eq!(back, DateTimeAsMicroseconds::create(2021, 5, 25, 17, 30, 3, 0));
assert_eq!(parts.to_date_time_as_microseconds(), Some(back));

// The day is not adjusted: Jan 31 + one month is Feb 31, which converts back to nothing
let mut end_of_month: DateTimeStruct = DateTimeAsMicroseconds::create(2021, 1, 31, 0, 0, 0, 0).into();
end_of_month.inc_month();
assert_eq!(end_of_month.to_date_time_as_microseconds(), None);

let same_day: DateTimeStruct = DateTimeAsMicroseconds::create(2021, 5, 25, 0, 0, 0, 0).into();
assert!(parts.is_date_the_same(&same_day));

// from_str: a date, a date-time, 14 compact digits, or the X.509 form
let parsed = DateTimeStruct::from_str("2021-04-25T17:30:03Z").unwrap();
assert_eq!(parsed.time.hour, 17);
assert!(DateTimeStruct::from_str("2021-04-25").is_some());
assert!(DateTimeStruct::from_str("20210425173003").is_some());
assert!(DateTimeStruct::from_str("Apr 25 17:30:03 2021 GMT").is_some());
```

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

// A known fixed offset: UTC+3 is 180 minutes
let utc_plus_3 = DateTimeAsMicrosecondsWithTimeZone::new(server, TimeZone::from_minutes(180));
assert_eq!(utc_plus_3.to_local_date_time_struct().time.hour, 20);
assert_eq!(format!("{:?}", utc_plus_3.time_zone), "+03:00");

let utc_minus_5 = DateTimeAsMicrosecondsWithTimeZone::new(server, TimeZone::from_minutes(-300));
assert_eq!(format!("{:?}", utc_minus_5.time_zone), "-05:00");
```

Serde writes and reads exactly `to_rfc3339()`. A bare number, or a string without an offset, is refused.

## Client time — a time the user typed in

The opposite direction of the section above: not showing a UTC instant in the user's zone, but reading a time the user typed on their own clock. `client_input_time_to_server_time` shifts it by the client/server difference, rounded to 30 minutes:

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
