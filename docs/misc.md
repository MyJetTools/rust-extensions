# misc

Paths, ids and small conveniences.

## file_utils

`format_path` expands a leading `~` into `$HOME`. A `~` anywhere else is part of a name and is kept. `FilePath` builds a path segment by segment with the platform separator.

```rust
use rust_extensions::file_utils::{format_path, FilePath};

let home = std::env::var("HOME").unwrap();
assert_eq!(format_path("~/.my-app").as_str(), format!("{}/.my-app", home));
assert_eq!(format_path("/data/report.txt~").as_str(), "/data/report.txt~");

let mut path = FilePath::from_str("/var/lib"); // expands `~` the same way
path.append_segment("my-app");
path.append_segment("/data/"); // stray separators are fine
assert_eq!(path.as_str(), "/var/lib/my-app/data");

assert_eq!(path.remove_segment(), Some("data".to_string()));
assert_eq!(path.as_str(), "/var/lib/my-app");

let _home_dir = FilePath::new_home();
let from_str: FilePath = "/tmp".into();
let _owned: String = from_str.into_string();
```

## uuid (feature `rnd`)

A random uuid-v4, lowercase and hyphenated. On wasm it comes from `crypto.randomUUID()`.

```rust
let id = rust_extensions::uuid::generate_v4();
assert_eq!(id.len(), 36);
assert_eq!(id.chars().filter(|c| *c == '-').count(), 4);
```

## SortableId (feature `rnd`)

`<unix microseconds>-<random>`, 25 characters. Ids sort by creation time as plain strings, so it works as a primary key that keeps insertion order. Serde writes it as the bare string.

```rust
use rust_extensions::SortableId;

let first = SortableId::generate();
std::thread::sleep(std::time::Duration::from_millis(1));
let second = SortableId::generate();

assert_eq!(first.as_str().len(), 25);
assert!(first < second);
assert!(first.as_str() < second.as_str());

let restored: SortableId = first.to_string().into();
assert_eq!(restored, first);
```

## DataWrapper

A derive for a newtype over a `Copy` value or an `Arc<String>`. It adds `new()`, an accessor and `Into` from the inner type. It does not work over a plain `String`.

```rust
use std::sync::Arc;
use rust_extensions::macros::DataWrapper;

#[derive(DataWrapper, Clone, Copy, Debug, PartialEq)]
pub struct AccountId(i64);

let id = AccountId::new(7);
assert_eq!(id.as_i64(), 7); // as_<type>
assert_eq!(*id.as_ref(), 7);
let from_inner: AccountId = 7i64.into();
assert_eq!(from_inner, id);

#[derive(DataWrapper)]
pub struct ClientName(Arc<String>);

let name: ClientName = "acme".to_string().into(); // from String or Arc<String>
assert_eq!(name.as_str(), "acme");
assert_eq!(name.to_string(), "acme");
```

## Re-exports

`rust_extensions::chrono` is the `chrono` this crate is built with — use it to name the types that `DateTimeAsMicroseconds::to_chrono_utc()` returns without a version mismatch. `rust_extensions::macros` holds the derives.

```rust
use rust_extensions::chrono::{DateTime, Utc};
use rust_extensions::date_time::DateTimeAsMicroseconds;

let utc: DateTime<Utc> = DateTimeAsMicroseconds::new(0).to_chrono_utc();
assert_eq!(utc.timestamp(), 0);
```
