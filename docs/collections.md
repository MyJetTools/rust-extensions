# collections

Grouping, containers that allocate only when used, capacity that shrinks back, running min/max, a stack-first vector, and an object pool.

## grouped_data

`group_to_hash_map` / `group_to_btree_map` collect an iterator into `Vec`s by key.

`GroupedDataAsHashmap` / `GroupedDataAsBTreeMap` keep values as group key → key → value. A group disappears once its last value is removed.

```rust
use rust_extensions::grouped_data::*;

struct Deal {
    account: String,
    id: u64,
}

let deals = vec![
    Deal { account: "a".into(), id: 1 },
    Deal { account: "b".into(), id: 2 },
    Deal { account: "a".into(), id: 3 },
];

let by_account = group_to_hash_map(deals.iter(), |deal| &deal.account);
assert_eq!(by_account["a"].len(), 2);

let by_account = group_to_btree_map(deals.into_iter(), |deal| &deal.account);
assert_eq!(by_account.keys().collect::<Vec<_>>(), vec!["a", "b"]);

let mut grouped = GroupedDataAsHashmap::new();
grouped.insert(&"a".to_string(), 1u64, "first");
grouped.insert(&"a".to_string(), 2u64, "second");
assert_eq!(grouped.get_data_by_group(&"a".to_string()).unwrap().len(), 2);

grouped.remove(&"a".to_string(), &1);
grouped.remove(&"a".to_string(), &2);
assert!(grouped.get_data_by_group(&"a".to_string()).is_none());
```

## lazy

Collect into a container that is allocated on the first insert only. `get_result()` is `None` when nothing was added — handy for "return `None` instead of an empty vec".

```rust
use rust_extensions::lazy::*;

let mut evens = LazyVec::with_capacity(16); // or LazyVec::new()
for i in [1, 3, 5] {
    if i % 2 == 0 {
        evens.add(i);
    }
}
assert!(evens.is_empty());
assert_eq!(evens.get_result(), None);

let mut map = LazyHashMap::new();
map.insert("k", 1);
assert_eq!(map.get_result().unwrap()["k"], 1);

let mut groups = LazyGroupIntoHashMap::new();
groups.add(&"a", 1);
groups.add(&"a", 2);
assert_eq!(groups.get_result().unwrap()["a"], vec![1, 2]);

let mut sorted_groups = LazyGroupIntoBTreeMap::new();
sorted_groups.add(&2, "two");
sorted_groups.add(&1, "one");
assert_eq!(sorted_groups.get_result().unwrap().keys().copied().collect::<Vec<_>>(), vec![1, 2]);
```

## linq

Turn a `Vec` into a map, skipping the items the converter answers `None` for. The `to_hash_map` / `to_btree_map` trait methods need the map value to be the item type, and a type annotation on the closure parameter. For any other value type use the converter directly.

```rust
use rust_extensions::linq::*;

#[derive(Clone)]
struct User {
    id: u64,
    active: bool,
}

let users = vec![User { id: 1, active: true }, User { id: 2, active: false }];

let active = users.clone().to_hash_map(|user: User| user.active.then(|| (user.id, user))).collect();
assert_eq!(active.len(), 1);

let sorted = users.clone().to_btree_map(|user: User| Some((user.id, user))).collect();
assert_eq!(sorted.keys().copied().collect::<Vec<_>>(), vec![1, 2]);

let flags = ToHashMapConverter::new(users, |user| Some((user.id, user.active))).collect();
assert_eq!(flags[&2], false);
```

## auto_shrink

`VecAutoShrink` / `VecDequeAutoShrink` give the capacity back after a spike. After a `pop`, `remove`, `clear` or `retain` leaves fewer items than `auto_shrink_capacity`, the capacity drops back to `auto_shrink_capacity`.

```rust
use rust_extensions::auto_shrink::{VecAutoShrink, VecDequeAutoShrink};

let mut buffer = VecAutoShrink::new(16);
for i in 0..10_000 {
    buffer.push(i);
}
buffer.clear();
assert!(buffer.capacity() < 10_000);

let mut queue = VecDequeAutoShrink::new(16);
queue.push_back(1);
queue.push_front(0);
assert_eq!(queue.pop_front(), Some(0));
queue.retain(|item| *item > 1);
assert!(queue.is_empty());

let with_items = VecAutoShrink::new_with_elements(16, [1, 2, 3].into_iter());
assert_eq!(with_items.iter().sum::<i32>(), 6);
```

## MinValue / MaxValue / MinKeyValue

A running minimum or maximum. It is `None` until the first `update`.

```rust
use rust_extensions::{MaxValue, MinKeyValue, MinValue};

let mut min = MinValue::new();
let mut max = MaxValue::new();
let mut cheapest = MinKeyValue::new();

for (price, venue) in [(1.5, "a"), (0.9, "b"), (2.0, "c")] {
    min.update(price);
    max.update(price);
    cheapest.update(price, venue);
}

assert_eq!(min.get_value(), Some(0.9));
assert_eq!(max.get_value(), Some(2.0));
assert_eq!(cheapest.get_value(), Some((0.9, "b")));
```

## VecMaybeStack (feature `vec-maybe-stack`)

The first N items live in an inline array, the rest go to a `Vec`. N comes from the buffer type: `Buffer32`, `Buffer128`, `Buffer256`, `Buffer512`, `Buffer1K`, `Buffer2K`, `Buffer4K`. Items must be `Copy + Default`.

```rust
use rust_extensions::vec_maybe_stack::{Buffer32, VecMaybeStack};

let mut bytes: VecMaybeStack<u8, Buffer32<u8>> = VecMaybeStack::new();
bytes.push(1);
bytes.push_slice(&[2; 40]); // 31 more fit on the stack, 9 spill to the heap

assert_eq!(bytes.len(), 41);
assert_eq!(bytes.iter().count(), 41);
assert_eq!(bytes.to_vec()[..2], [1, 2]);
```

## ObjectsPool (feature `objects-pool`)

A pool of up to `max_pool_size` expensive objects, such as connections or big buffers. `get_element()` hands out a `RentedObject`. Dropping it returns the object to the pool. When every object is rented, `get_element()` waits, re-checking every 100 ms.

```rust
use std::sync::Arc;
use rust_extensions::objects_pool::{ObjectsPool, ObjectsPoolFactory};

struct BufferFactory;

#[async_trait::async_trait]
impl ObjectsPoolFactory<Vec<u8>> for BufferFactory {
    async fn create_new(&self) -> Vec<u8> {
        Vec::with_capacity(1024 * 1024)
    }
}

async fn handle(pool: &ObjectsPool<Vec<u8>, BufferFactory>) {
    let buffer = pool.get_element().await; // created on demand, up to the limit
    let capacity = buffer.get_value().capacity(); // or buffer.as_ref()
    assert!(capacity >= 1024 * 1024);
} // the buffer goes back to the pool here

fn create_pool() -> ObjectsPool<Vec<u8>, BufferFactory> {
    ObjectsPool::new(4, Arc::new(BufferFactory))
}
```
