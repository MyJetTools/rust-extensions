//! `date_time` is written without chrono, but it replaced chrono: what chrono wrote is still out
//! there in MyNoSql entities, jsonb columns and Service Bus messages, and every other participant
//! already speaks that format. So chrono stays as a dev-dependency and each value below goes
//! through both implementations, which have to agree byte for byte.
//!
//! The left side of every comparison is this crate, the right side is what the crate did when it
//! was calling chrono.

use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, SecondsFormat, Timelike, Utc};
use rust_extensions::date_time::{
    DateTimeAsMicroseconds, DateTimeAsMicrosecondsWithTimeZone, DateTimeStruct, TimeStruct,
    TimeZone,
};

const MICROS_IN_ONE_DAY: i64 = 86_400_000_000;

/// A fixed seed, so a failure repeats.
struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn in_range(&mut self, from: i64, to: i64) -> i64 {
        let width = to.wrapping_sub(from) as u64 + 1;
        from.wrapping_add((self.next() % width) as i64)
    }
}

fn assert_fields_match<Tz: chrono::TimeZone>(fields: &DateTimeStruct, expected: &DateTime<Tz>) {
    let got = (
        fields.year,
        fields.month,
        fields.day,
        fields.time.hour,
        fields.time.min,
        fields.time.sec,
        fields.time.micros,
    );

    let wanted = (
        expected.year(),
        expected.month(),
        expected.day(),
        expected.hour(),
        expected.minute(),
        expected.second(),
        expected.nanosecond() / 1000,
    );

    assert_eq!(got, wanted);

    let day_of_week = fields.get_day_of_week();
    let wanted = expected.weekday();

    assert_eq!(day_of_week.num_days_from_monday(), wanted.num_days_from_monday());
    assert_eq!(day_of_week.num_days_from_sunday(), wanted.num_days_from_sunday());
}

fn assert_instant_matches(unix_microseconds: i64) {
    let date_time = DateTimeAsMicroseconds::new(unix_microseconds);
    let chrono = DateTime::<Utc>::from_timestamp_micros(unix_microseconds).unwrap();

    assert_eq!(date_time.to_rfc3339(), chrono.to_rfc3339());
    assert_eq!(
        date_time.to_rfc3339_utc(),
        chrono.to_rfc3339_opts(SecondsFormat::Micros, true)
    );

    let fields: DateTimeStruct = date_time.into();
    assert_fields_match(&fields, &chrono);
}

fn assert_zone_matches(unix_microseconds: i64, offset_in_minutes: i32) {
    let time_zone = TimeZone::from_minutes(offset_in_minutes);
    let value = DateTimeAsMicrosecondsWithTimeZone::new(
        DateTimeAsMicroseconds::new(unix_microseconds),
        time_zone,
    );

    // The removed `TimeZone::to_fixed_offset`
    let fixed_offset = FixedOffset::east_opt(offset_in_minutes * 60)
        .unwrap_or_else(|| FixedOffset::east_opt(0).unwrap());

    let chrono = DateTime::<Utc>::from_timestamp_micros(unix_microseconds)
        .unwrap()
        .with_timezone(&fixed_offset);

    assert_eq!(
        value.to_rfc3339(),
        chrono.to_rfc3339_opts(SecondsFormat::Micros, false)
    );
    assert_eq!(format!("{:?}", time_zone), fixed_offset.to_string());
    assert_fields_match(&value.to_local_date_time_struct(), &chrono);
}

fn chrono_to_unix_microseconds(src: &DateTimeStruct) -> Option<i64> {
    let date_time = NaiveDate::from_ymd_opt(src.year, src.month, src.day)?.and_hms_micro_opt(
        src.time.hour,
        src.time.min,
        src.time.sec,
        src.time.micros,
    )?;

    Some(date_time.and_utc().timestamp_micros())
}

fn assert_parsing_matches(src: &str) {
    let parsed = DateTimeAsMicrosecondsWithTimeZone::from_str(src)
        .map(|v| (v.date_time.unix_microseconds, v.time_zone.offset_in_minutes()));

    let chrono = DateTime::parse_from_rfc3339(src)
        .ok()
        .map(|v| (v.timestamp_micros(), v.offset().local_minus_utc() / 60));

    assert_eq!(parsed, chrono, "parsing {:?}", src);
}

/// `from_str` is the tolerant reader and takes more than chrono does, so only what chrono reads is
/// compared: that has to be the same moment, plain and URL-encoded.
fn assert_from_str_matches(src: &str) {
    let Ok(chrono) = DateTime::parse_from_rfc3339(src) else {
        return;
    };

    let expected = Some(chrono.timestamp_micros());
    let url_encoded = src.replace(':', "%3A").replace('+', "%2B");

    for src in [src, url_encoded.as_str()] {
        let parsed = DateTimeAsMicroseconds::from_str(src).map(|v| v.unix_microseconds);

        assert_eq!(parsed, expected, "parsing {:?}", src);
    }
}

#[test]
fn instants_around_the_epoch() {
    for unix_microseconds in -3_000..=3_000 {
        assert_instant_matches(unix_microseconds);
    }

    for second in -3_000..=3_000 {
        assert_instant_matches(second * 1_000_000);
        assert_instant_matches(second * 1_000_000 - 1);
        assert_instant_matches(second * 1_000_000 + 1_000);
    }
}

#[test]
fn the_first_and_the_last_moment_of_a_day() {
    // 1900-01-01 to 2100-01-01
    for day in -25_567..=47_482 {
        assert_instant_matches(day * MICROS_IN_ONE_DAY);
        assert_instant_matches(day * MICROS_IN_ONE_DAY - 1);
    }
}

#[test]
fn random_instants() {
    let mut random = XorShift(0x9E37_79B9_7F4A_7C15);

    for _ in 0..100_000 {
        // 1900 to 2200, with the fraction that 0, 3 and 6 digits are chosen by
        let unix_microseconds = random.in_range(-2_208_988_800_000_000, 7_258_118_400_000_000);

        assert_instant_matches(unix_microseconds);
        assert_instant_matches(unix_microseconds / 1_000 * 1_000);
        assert_instant_matches(unix_microseconds / 1_000_000 * 1_000_000);

        // Nearly all the years chrono can hold, the ones which take a sign among them
        let far_away = random.in_range(-8_200_000_000_000_000_000, 8_200_000_000_000_000_000);
        assert_instant_matches(far_away);
    }
}

#[test]
fn zones() {
    let mut random = XorShift(0x2545_F491_4F6C_DD1D);

    // Past ±24h, so the fallback to UTC is covered too.
    for offset_in_minutes in -1_500..=1_500 {
        for _ in 0..20 {
            let unix_microseconds = random.in_range(-2_208_988_800_000_000, 7_258_118_400_000_000);
            assert_zone_matches(unix_microseconds, offset_in_minutes);
        }

        assert_zone_matches(0, offset_in_minutes);
    }
}

#[test]
fn dates_are_validated_the_same_way() {
    let years = (-500..=3_000).chain([
        i32::MIN,
        -262_144,
        -262_143,
        -262_142,
        262_141,
        262_142,
        262_143,
        i32::MAX,
    ]);

    for year in years {
        for month in 0..=13 {
            for day in 0..=32 {
                let src = DateTimeStruct {
                    year,
                    month,
                    day,
                    time: TimeStruct {
                        hour: 12,
                        min: 34,
                        sec: 56,
                        micros: 789_012,
                    },
                    dow: None,
                };

                assert_eq!(
                    src.to_unix_microseconds(),
                    chrono_to_unix_microseconds(&src),
                    "{}-{}-{}",
                    year,
                    month,
                    day
                );
            }
        }
    }
}

#[test]
fn times_are_validated_the_same_way() {
    for hour in 0..=24 {
        for min in 0..=60 {
            for sec in 0..=60 {
                for micros in [0, 999_999, 1_000_000, 1_999_999, 2_000_000] {
                    let src = DateTimeStruct {
                        year: 2021,
                        month: 4,
                        day: 25,
                        time: TimeStruct {
                            hour,
                            min,
                            sec,
                            micros,
                        },
                        dow: None,
                    };

                    assert_eq!(
                        src.to_unix_microseconds(),
                        chrono_to_unix_microseconds(&src),
                        "{}:{}:{}.{}",
                        hour,
                        min,
                        sec,
                        micros
                    );
                }
            }
        }
    }
}

#[test]
fn spellings_of_a_date_time_with_an_offset() {
    for src in [
        "2021-04-25T18:30:03+01:00",
        "2021-04-25T18:30:03-05:00",
        "2021-04-25T18:30:03\u{2212}05:00",
        "2021-04-25t18:30:03z",
        "2021-04-25 18:30:03Z",
        "2021-04-25T18:30:03.1Z",
        "2021-04-25T18:30:03.123456Z",
        "2021-04-25T18:30:03.123456789Z",
        "2021-04-25T18:30:03.123456789012Z",
        "2021-04-25T18:30:03.Z",
        "2016-12-31T23:59:60Z",
        "2016-12-31T23:59:60.5+01:00",
        "2021-04-25T12:34:60Z",
        "2021-04-25T24:00:00Z",
        "2021-04-25T18:60:03Z",
        "2021-04-25T18:30:61Z",
        "2021-02-29T00:00:00Z",
        "2020-02-29T00:00:00Z",
        "2021-13-01T00:00:00Z",
        "2021-00-01T00:00:00Z",
        "0000-01-01T00:00:00Z",
        "0000-01-01T00:00:00+23:59",
        "9999-12-31T23:59:59.999999Z",
        "9999-12-31T23:59:59.999999-23:59",
        "2021-04-25T18:30:03+24:00",
        "2021-04-25T18:30:03+23:59",
        "2021-04-25T18:30:03-23:59",
        "2021-04-25T18:30:03+01:60",
        "2021-04-25T18:30:03+0100",
        "2021-04-25T18:30:03+01",
        "2021-04-25T18:30:03+",
        "2021-04-25T18:30:03",
        "2021-04-25T18:30",
        "2021-04-25",
        "",
        "2021-04-25T18:30:03Zjunk",
        "2021-04-25T18:30:03+01:00 ",
        " 2021-04-25T18:30:03Z",
        "2021-04-25T18:30:03é",
        "2021-04-25T18:30:03.é",
        "２０２１-04-25T18:30:03Z",
        "not-a-date-not-a-date-not-a-date",
    ] {
        assert_parsing_matches(src);
        assert_from_str_matches(src);
    }
}

#[test]
fn mutated_date_times_with_an_offset() {
    let mut random = XorShift(0xD1B5_4A32_D192_ED03);
    let alphabet: Vec<char> = "0123456789-+:.TtZz \u{2212}".chars().collect();

    for _ in 0..300_000 {
        // Years 0000 to 9999
        let unix_microseconds = random.in_range(-62_000_000_000_000_000, 253_000_000_000_000_000);
        let separator = ['T', 't', ' '][random.in_range(0, 2) as usize];

        let mut src = DateTimeAsMicroseconds::new(unix_microseconds).to_rfc3339_utc();
        src.truncate(19);
        src.replace_range(10..11, separator.to_string().as_str());

        let digits = random.in_range(0, 12);

        if digits > 0 {
            src.push('.');

            for _ in 0..digits {
                src.push(char::from(b'0' + random.in_range(0, 9) as u8));
            }
        }

        match random.in_range(0, 4) {
            0 => src.push('Z'),
            1 => src.push('z'),
            variant => {
                let sign = ['+', '-', '\u{2212}'][variant as usize - 2];
                let hours = random.in_range(0, 25);
                let minutes = random.in_range(0, 62);
                src.push_str(format!("{}{:02}:{:02}", sign, hours, minutes).as_str());
            }
        }

        let mut chars: Vec<char> = src.chars().collect();

        for _ in 0..random.in_range(0, 3) {
            if chars.is_empty() {
                break;
            }

            let position = random.in_range(0, chars.len() as i64 - 1) as usize;
            let new_char = alphabet[random.in_range(0, alphabet.len() as i64 - 1) as usize];

            match random.in_range(0, 3) {
                0 => chars[position] = new_char,
                1 => chars.insert(position, new_char),
                2 => {
                    chars.remove(position);
                }
                _ => chars.truncate(position + 1),
            }
        }

        let src = chars.into_iter().collect::<String>();

        assert_parsing_matches(src.as_str());
        assert_from_str_matches(src.as_str());
    }
}
