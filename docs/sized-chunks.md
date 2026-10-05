# sized-chunks

Cut a collection into batches by measured size, not by item count.

Real limits are in bytes: a gRPC message, a request body, a datagram, a row batch handed to a driver. "Every 100 items" is a guess about the average item, and it fails exactly when the items are unusually large. The caller supplies the cost function, so the same helper works with `prost::Message::encoded_len`, `serde_json::to_vec(..).len()`, `str::len` or your own estimate.

## split_into_sized_chunks

```rust
use rust_extensions::split_into_sized_chunks;

let rows = vec!["four", "five5", "six666"];

let chunks = split_into_sized_chunks(rows, 10, |row| row.len());
assert_eq!(chunks, vec![vec!["four", "five5"], vec!["six666"]]);

// The limit is inclusive: a chunk of exactly `limit` is fine.
let chunks = split_into_sized_chunks(vec!["12345", "67890"], 10, |row| row.len());
assert_eq!(chunks, vec![vec!["12345", "67890"]]);

// An item bigger than the whole limit becomes a chunk of its own - the real
// boundary rejects it with its own error instead of the batcher spinning.
let chunks = split_into_sized_chunks(vec!["x".repeat(50), "y".to_string()], 10, |row| row.len());
assert_eq!(chunks.len(), 2);

// Empty input - no chunks at all, so "one message per chunk" sends nothing.
assert!(split_into_sized_chunks(Vec::<String>::new(), 10, |row| row.len()).is_empty());
```

A gRPC stream, one message per chunk, each under a 4 MiB decode limit. The `+ 8` stands for the tag and the length prefix a `repeated` item adds on the wire, and 3 MiB leaves headroom for the rest of the message:

```rust,ignore
for page in split_into_sized_chunks(rows, 3 * 1024 * 1024, |row| row.encoded_len() + 8) {
    producer.send(Response { page }).await?;
}
```

## SizeBudget

The counter behind it. Drive it yourself when one batch holds items of several types, for example two `repeated` fields of one protobuf message:

```rust
use rust_extensions::SizeBudget;

#[derive(Default)]
struct Page {
    deals: Vec<String>,
    orders: Vec<String>,
}

let deals = vec!["d".repeat(6), "d".repeat(6)];
let orders = vec!["o".repeat(3)];

let mut pages = Vec::new();
let mut page = Page::default();
let mut budget = SizeBudget::new(10);

for deal in deals {
    let cost = deal.len();
    if budget.needs_flush(cost) {
        pages.push(std::mem::take(&mut page));
        budget.reset();
    }
    budget.add(cost);
    page.deals.push(deal);
}

for order in orders {
    let cost = order.len();
    if budget.needs_flush(cost) {
        pages.push(std::mem::take(&mut page));
        budget.reset();
    }
    budget.add(cost);
    page.orders.push(order);
}

if !budget.is_empty() {
    pages.push(page);
}

assert_eq!(pages.len(), 2);
assert_eq!((pages[1].deals.len(), pages[1].orders.len()), (1, 1)); // the types share a page
assert_eq!((budget.used(), budget.limit()), (9, 10));
```

`needs_flush` is always `false` on an empty batch. That is why an oversized item becomes a batch of one without a special case at every call site.
