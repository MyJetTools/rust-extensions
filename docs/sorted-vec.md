# sorted-vec

A `Vec` kept sorted by a key taken from the item itself. Lookups are binary searches, and a range of keys is a slice. It is the in-memory index for caches and order books.

| Type | Key | Items |
| --- | --- | --- |
| `sorted_vec::SortedVec<K, T>` | `T: EntityWithKey<K>`, `K: Ord` | `T` |
| `sorted_vec::SortedVecWithStrKey<T>` | `T: EntityWithStrKey` | `T` |
| `sorted_vec::SortedVecOfArc<K, T>` | `T: EntityWithKey<K>` | `Arc<T>` |
| `sorted_vec::SortedVecOfArcWithStrKey<T>` | `T: EntityWithStrKey` | `Arc<T>` |
| `SortedVecWith2StrKey<T>` | `T: EntityWith2StrKey` | `T`, partitioned by the primary key |
| `SortedVecOfArcWith2StrKey<T>` | `T: EntityWith2StrKey` | `Arc<T>`, partitioned by the primary key |

The single-key types share one API; examples use `SortedVec`.

## Insert, look up, remove

```rust
use rust_extensions::sorted_vec::{EntityWithKey, SortedVec};

#[derive(Clone, Debug)]
struct Order {
    id: u64,
    amount: f64,
}

impl EntityWithKey<u64> for Order {
    fn get_key(&self) -> &u64 {
        &self.id
    }
}

let mut orders = SortedVec::new(); // also new_with_capacity, from_iterator (sorts)

let (index, replaced) = orders.insert_or_replace(Order { id: 20, amount: 1.0 });
assert_eq!((index, replaced.is_none()), (0, true));
orders.insert_or_replace(Order { id: 10, amount: 2.0 });
let (_, replaced) = orders.insert_or_replace(Order { id: 20, amount: 3.0 });
assert_eq!(replaced.unwrap().amount, 1.0);

assert!(orders.contains(&10));
assert_eq!(orders.get(&20).unwrap().amount, 3.0);
orders.get_mut(&20).unwrap().amount = 4.0;
assert_eq!(orders.get_by_index(0).unwrap().id, 10);
assert_eq!(orders.first().unwrap().id, 10);
assert_eq!(orders.last().unwrap().id, 20);

let ids: Vec<u64> = orders.iter().map(|order| order.id).collect();
assert_eq!(ids, vec![10, 20]);

assert_eq!(orders.remove(&10).unwrap().id, 10);
assert!(orders.remove_at(5).is_none());
assert_eq!(orders.len(), 1);
```

## Entries — one search, then insert or update

An entry carries the position the binary search found, so the item is not searched for twice. Insert an item with the same key you asked for, or the order breaks.

```rust
use rust_extensions::sorted_vec::*;

#[derive(Clone)]
struct Balance {
    account: u64,
    value: f64,
}

impl EntityWithKey<u64> for Balance {
    fn get_key(&self) -> &u64 {
        &self.account
    }
}

let mut balances = SortedVec::new();

match balances.insert_or_update(&7) {
    InsertOrUpdateEntry::Insert(entry) => entry.insert(Balance { account: 7, value: 10.0 }),
    InsertOrUpdateEntry::Update(entry) => entry.item.value += 10.0,
}

match balances.get_mut_or_create(&7) {
    GetMutOrCreateEntry::GetMut(balance) => balance.value += 5.0,
    GetMutOrCreateEntry::Create(entry) => entry.insert(Balance { account: 7, value: 5.0 }),
}
assert_eq!(balances.get(&7).unwrap().value, 15.0);

match balances.insert_or_if_not_exists(&8) {
    InsertIfNotExists::Insert(entry) => {
        let inserted = entry.insert_and_get_value(Balance { account: 8, value: 0.0 });
        assert_eq!(inserted.account, 8);
    }
    InsertIfNotExists::Exists(index) => panic!("already at {}", index),
}
```

The `Arc` flavours have `get_or_create`, which answers with `GetOrCreateEntry::Get(&Arc<T>)` or `Create(entry)`, instead of `get_mut_or_create`.

## Ranges and windows

```rust
use rust_extensions::sorted_vec::{EntityWithKey, SortedVec};

#[derive(Clone)]
struct Tick(i64);

impl EntityWithKey<i64> for Tick {
    fn get_key(&self) -> &i64 {
        &self.0
    }
}

let ticks = SortedVec::from_iterator([10, 20, 30, 40, 50].map(Tick));
let keys = |slice: &[Tick]| slice.iter().map(|tick| tick.0).collect::<Vec<_>>();

// range(from..to) INCLUDES `to` when an item with that key exists
assert_eq!(keys(&ticks.range(20..40)), vec![20, 30, 40]);
assert_eq!(keys(&ticks.range(15..35)), vec![20, 30]);
let copy: SortedVec<i64, Tick> = ticks.sub_sequence(20..40); // the same range, cloned

assert_eq!(keys(&ticks.range_by_index(1..3)), vec![20, 30]);

// From a key (inclusive) up / down
assert_eq!(keys(ticks.get_from_key_to_up(&30)), vec![30, 40, 50]);
assert_eq!(keys(ticks.get_from_bottom_to_key(&30)), vec![10, 20, 30]);

// The last `amount` items not above a key - "the 2 candles up to 35"
assert_eq!(keys(ticks.get_highest_and_below_amount(&35, 2)), vec![20, 30]);
```

## Draining and capacity

Three names that do not quite say what they do:

```rust
use rust_extensions::sorted_vec::{EntityWithKey, SortedVec};

#[derive(Clone)]
struct Tick(i64);

impl EntityWithKey<i64> for Tick {
    fn get_key(&self) -> &i64 {
        &self.0
    }
}

let keys = |items: &[Tick]| items.iter().map(|tick| tick.0).collect::<Vec<_>>();

let mut ticks = SortedVec::from_iterator([10, 20, 30].map(Tick));
assert_eq!(keys(&ticks.drain_into_vec()), vec![30, 20, 10]); // descending - it pops

let mut ticks = SortedVec::from_iterator([10, 20, 30].map(Tick));
ticks.truncate_capacity(2); // keeps the first 2 items
assert_eq!(keys(ticks.as_slice()), vec![10, 20]);

ticks.clear(Some(1024)); // empties; Some(n) makes the capacity at least n
assert!(ticks.is_empty() && ticks.capacity() >= 1024);

let mut ticks = SortedVec::from_iterator([10, 20, 30].map(Tick));
assert_eq!(ticks.pop().unwrap().0, 30); // the highest key
assert_eq!(keys(&ticks.into_vec()), vec![10, 20]);
```

## Two string keys

`SortedVecWith2StrKey` keeps one sorted partition per primary key and sorts the rows inside it by the secondary key. For example: positions by account, then by instrument.

```rust
use rust_extensions::{EntityWith2StrKey, InsertOrUpdateEntry2Keys, SortedVecWith2StrKey};

struct Position {
    account: String,
    instrument: String,
    volume: f64,
}

impl EntityWith2StrKey for Position {
    fn get_primary_key(&self) -> &str {
        &self.account
    }

    fn get_secondary_key(&self) -> &str {
        &self.instrument
    }
}

let position = |account: &str, instrument: &str, volume: f64| Position {
    account: account.to_string(),
    instrument: instrument.to_string(),
    volume,
};

let mut positions = SortedVecWith2StrKey::new();
positions.insert_or_replace(position("acc-1", "EURUSD", 1.0));
positions.insert_or_replace(position("acc-1", "BTCUSD", 0.5));
positions.insert_or_replace(position("acc-2", "EURUSD", 2.0));

assert_eq!(positions.len(), 3);
assert_eq!(positions.partitions_len(), 2);
assert_eq!(positions.get("acc-1", "EURUSD").unwrap().volume, 1.0);
assert!(positions.contains("acc-2", "EURUSD"));

// One partition, sorted by the secondary key
let instruments: Vec<&str> = positions
    .get_by_primary_key("acc-1")
    .unwrap()
    .map(|p| p.instrument.as_str())
    .collect();
assert_eq!(instruments, vec!["BTCUSD", "EURUSD"]);

assert_eq!(positions.range("acc-1", "A".."C").unwrap().len(), 1);
assert_eq!(positions.get_from_key_to_up("acc-1", "C").unwrap().len(), 1);

match positions.insert_or_update("acc-2", "EURUSD") {
    InsertOrUpdateEntry2Keys::Insert(entry) => entry.insert(position("acc-2", "EURUSD", 1.0)),
    InsertOrUpdateEntry2Keys::Update(mut entry) => entry.get_item_mut().volume += 1.0,
}
assert_eq!(positions.get("acc-2", "EURUSD").unwrap().volume, 3.0);

assert!(positions.remove("acc-2", "EURUSD").is_some());
assert_eq!(positions.partitions_len(), 1); // an emptied partition goes away

let removed = positions.remove_by_primary_key("acc-1").unwrap();
assert_eq!(removed.len(), 2);
assert!(positions.is_empty());
```
