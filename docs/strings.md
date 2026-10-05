# strings

Small strings without a heap allocation, borrow-or-own strings, a builder, and case-insensitive helpers.

## ShortString

Up to 255 bytes of UTF-8 inline: a `[u8; 256]` with the length in the first byte. `Clone`, comparable with `&str`, `Deref<Target = str>`. There is no `Hash` and no serde.

`Ord` is by length first, then by bytes: `"b" < "aa"`. Fine for a map key, not for alphabetical output.

The `push*` methods panic past 255 bytes; the `try_*` ones return `false` instead.

```rust
use rust_extensions::ShortString;

let mut s = ShortString::from_str("hi").unwrap();
assert!(ShortString::from_str(&"x".repeat(256)).is_none());

s.push('-');
s.push_str("there");
assert!(s.try_push('!'));
assert!(s.try_push_str(" ok"));
assert_eq!(s.as_str(), "hi-there! ok");
assert_eq!(s, "hi-there! ok");
assert_eq!(s.len(), 12);

let mut full = ShortString::from_str(&"x".repeat(255)).unwrap();
assert!(!full.try_push('y'));
assert!(!full.try_push_str("y"));

s.update("Hello");
s.insert(5, " world").unwrap();
assert_eq!(s.as_str(), "Hello world");

assert!(s.replace("world", "there")); // false and unchanged when the result would not fit
assert_eq!(s.as_str(), "Hello there");

s.set_len(5); // cuts; panics inside a multi-byte char
assert_eq!(s.as_str(), "Hello");

assert!(s.compare_with_case_insensitive("HELLO"));
assert_eq!(ShortString::from_str_convert_to_lower_case("ABC").unwrap(), "abc");

assert!(s.starts_with("He")); // any `str` method through Deref
let borrowed: &str = &s;
let from_str: ShortString = "key".into(); // panics past 255 bytes
assert!(ShortString::from_str("b").unwrap() < ShortString::from_str("aa").unwrap());
```

## MaybeShortString

A `ShortString` while the value fits, a `String` once it does not. Pushing past 255 bytes switches it over by itself.

```rust
use rust_extensions::{MaybeShortString, ShortString, StrOrString};

let mut s = MaybeShortString::from_str("abc");
assert!(matches!(s, MaybeShortString::AsShortString(_)));

s.push_str(&"x".repeat(300));
assert!(matches!(s, MaybeShortString::AsString(_)));
assert_eq!(s.len(), 303);

let mut empty = MaybeShortString::new();
empty.push('a');
assert_eq!(empty.as_str(), "a");

assert_eq!(MaybeShortString::from_str_as_lower_case("AbC").as_str(), "abc");
assert_eq!(MaybeShortString::from_str_as_upper_case("AbC").as_str(), "ABC");

let short: Result<ShortString, String> = MaybeShortString::from_str("abc").try_into();
assert!(short.is_ok());

// The inherent `.into()` gives a StrOrString and shadows `Into<String>`
let as_str_or_string: StrOrString = MaybeShortString::from_str("abc").into();
let as_string: String = Into::<String>::into(MaybeShortString::from_str("abc"));
```

## StrOrString

A `&'s str` or an owned `String` behind one type — for an API that usually gets a literal but sometimes has to build the value.

```rust
use rust_extensions::StrOrString;

fn name_of(name: impl Into<StrOrString<'static>>) -> String {
    let name: StrOrString<'static> = name.into();
    name.as_str().to_uppercase()
}

assert_eq!(name_of("static"), "STATIC");
assert_eq!(name_of(format!("built-{}", 1)), "BUILT-1");

let borrowed = StrOrString::create_as_str("abc");
let owned = StrOrString::create_as_string("abc".to_string());
assert_eq!(borrowed.as_str(), owned.as_str());

// A window into the value, by byte offsets
let mut window: StrOrString = "Hello world".into();
window.slice_it(Some(6), None);
assert_eq!(window.as_str(), "world");
assert_eq!(window.to_string(), "world"); // Display
assert_eq!(window.clone().into_string(), "world");
assert_eq!(window.to_short_string(), "world"); // panics past 255 bytes

assert!(owned.compare_with_case_insensitive("ABC"));
let string: String = owned.into();
```

## StringBuilder

```rust
use rust_extensions::StringBuilder;

let mut sb = StringBuilder::new();
sb.append_str("a");
sb.append_char('b');
sb.append_line("c"); // appends '\n'
sb.append_bytes("ü".as_bytes()).unwrap(); // UTF-8 checked
sb.append_byte(b'!'); // the byte as a char: Latin-1 above 0x7F

assert_eq!(sb.to_string_utf8(), "abc\nü!");
```

For a secret use [secure-string-builder](secure-string-builder.md).

## AsStr

One bound for "anything that can give a `&str`". It is implemented for:

- `String`, `&String`, `str` and `&str`
- `ShortString`
- `SecureStringBuilder`

```rust
use rust_extensions::{AsStr, ShortString};

fn log_key(key: &impl AsStr) -> usize {
    key.as_str().len()
}

assert_eq!(log_key(&"abc"), 3);
assert_eq!(log_key(&"abc".to_string()), 3);
assert_eq!(log_key(&ShortString::from_str("abc").unwrap()), 3);
```

## str_utils

Case-insensitive comparison works on ASCII only. The split helpers return `None` when the number of parts is not the one asked for.

```rust
use rust_extensions::str_utils::{compare_strings_case_insensitive, starts_with_case_insensitive, StrUtils};

assert!(compare_strings_case_insensitive("Content-Type", "content-type"));
assert!(starts_with_case_insensitive("Bearer abc", "bearer "));

assert!("ABC".eq_case_insensitive("abc"));
assert!("Bearer abc".starts_with_case_insensitive("BEARER"));

assert_eq!("host:443".split_exact_to_2_lines(":"), Some(("host", "443")));
assert_eq!("host".split_exact_to_2_lines(":"), None);
assert_eq!("a:b:c".split_exact_to_2_lines(":"), None);

assert_eq!("host".split_up_to_2_lines(":"), Some(("host", None)));
assert_eq!("host:443".split_up_to_2_lines(":"), Some(("host", Some("443"))));

assert_eq!("a.b.c".split_exact_to_3_lines("."), Some(("a", "b", "c")));
assert_eq!("a.b".split_2_or_3_lines("."), Some(("a", "b", None)));
assert_eq!("a.b.c".split_2_or_3_lines("."), Some(("a", "b", Some("c"))));
```
