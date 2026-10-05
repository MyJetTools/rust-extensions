# interval-keys

`rust_extensions::date_time` — cut timestamps into buckets (candles, daily stats, hourly reports) and use the bucket as a compact `i64` key.

A key is the slot start written as decimal digits, so the numbers sort the way the time does — within one key type. Keys of different types are not comparable to each other.

| Key type | Bucket | `i64` layout |
| --- | --- | --- |
| `YearKey` | year | `YYYY` |
| `MonthKey` | month | `YYYYMM` |
| `WeekMondayKey` / `WeekSundayKey` | week starting Monday / Sunday | `YYYYMMDD` of the week start |
| `DayKey` | day | `YYYYMMDD` |
| `HourKey`, `Hour2Key`, `Hour4Key` | 1h / 2h / 4h | `YYYYMMDDHH` of the slot start |
| `MinuteKey`, `Minute5Key`, `Minute15Key`, `Minute30Key` | 1m / 5m / 15m / 30m | `YYYYMMDDHHmm` of the slot start |

## IntervalKey — the key type checked at compile time

```rust
use std::time::Duration;
use rust_extensions::date_time::*;

let dt = DateTimeAsMicroseconds::create(2021, 4, 25, 17, 37, 3, 0);

let minute: IntervalKey<MinuteKey> = dt.into();
assert_eq!(minute.to_i64(), 202104251737);

assert_eq!(IntervalKey::<Minute15Key>::new(dt).to_i64(), 202104251730);
assert_eq!(IntervalKey::<Hour4Key>::new(dt).to_i64(), 2021042516);
assert_eq!(IntervalKey::<DayKey>::new(dt).to_i64(), 20210425);
assert_eq!(IntervalKey::<WeekMondayKey>::new(dt).to_i64(), 20210419);
assert_eq!(IntervalKey::<WeekSundayKey>::new(dt).to_i64(), 20210425);
assert_eq!(IntervalKey::<MonthKey>::new(dt).to_i64(), 202104);
assert_eq!(IntervalKey::<YearKey>::new(dt).to_i64(), 2021);

// Back to the slot start
assert_eq!(
    minute.try_to_date_time().unwrap(),
    DateTimeAsMicroseconds::create(2021, 4, 25, 17, 37, 0, 0)
);

// Step by the slot width - a shorter step snaps back to the same slot
let next = minute.add(Duration::from_secs(60));
assert_eq!(next.to_i64(), 202104251738);
assert_eq!(minute.add(Duration::from_secs(30)), minute);
assert_eq!(next.sub(Duration::from_secs(60)), minute);

// add/sub are not calendar-aware: they move the slot start by the Duration and snap.
// Months and years are not fixed Durations - step them one slot at a time:
let month: IntervalKey<MonthKey> = dt.into();
let day = Duration::from_secs(86_400);
assert_eq!(month.add(day * 31).to_i64(), 202105); // from the 1st, 31 days is always the next month
assert_eq!(month.sub(day).to_i64(), 202103); // a day before the 1st is the previous month
let year: IntervalKey<YearKey> = dt.into();
assert_eq!(year.add(day * 366).to_i64(), 2022);

// Keys are Copy + Ord + Hash
assert!(minute < next);

// From a stored i64 - unchecked: it has to be a key of the very same type
let restored: IntervalKey<MinuteKey> = 202104251737i64.into();
assert_eq!(restored, minute);
assert_eq!(IntervalKey::<MinuteKey>::from_i64(202104251737), minute);
```

## DateTimeInterval — the key type chosen at runtime

The same buckets as an enum, for when the interval comes from configuration or a request:

```rust
use rust_extensions::date_time::*;

let dt = DateTimeAsMicroseconds::create(2021, 4, 25, 17, 37, 3, 0);

let interval = DateTimeInterval::from_dt_to_min5(dt);
assert_eq!(interval, DateTimeInterval::Min5(202104251735));
assert_eq!(interval.to_i64(), 202104251735);
assert_eq!(
    interval.to_date_time().unwrap(),
    DateTimeAsMicroseconds::create(2021, 4, 25, 17, 35, 0, 0)
);

let key: IntervalKey<Minute5Key> = dt.into();
assert_eq!(key.to_dt_interval(), interval);

// One constructor per bucket
let _ = [
    DateTimeInterval::from_dt_to_minute(dt),
    DateTimeInterval::from_dt_to_min15(dt),
    DateTimeInterval::from_dt_to_min30(dt),
    DateTimeInterval::from_dt_to_hour(dt),
    DateTimeInterval::from_dt_to_hour2(dt),
    DateTimeInterval::from_dt_to_hour4(dt),
    DateTimeInterval::from_dt_to_day(dt),
    DateTimeInterval::from_dt_to_week_monday(dt),
    DateTimeInterval::from_dt_to_week_sunday(dt),
    DateTimeInterval::from_dt_to_month(dt),
    DateTimeInterval::from_dt_to_year(dt),
];
```
