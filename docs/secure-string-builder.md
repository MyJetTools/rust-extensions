# secure-string-builder

`SecureStringBuilder` — build a password, a token, a connection string or a private key without leaving copies of it in freed memory.

A plain `String` grows by `realloc`: the old block is freed as it is, so a secret that grew from 8 to 16 to 32 bytes leaves three readable copies in the heap. `SecureStringBuilder` never lets its buffer re-allocate itself. When the next push would not fit, it:

1. allocates a bigger buffer and copies the content over;
2. zeroes the whole old allocation — its full capacity, not just the used length — with volatile writes the optimiser can not drop;
3. frees the old allocation.

`Drop` and `clear()` wipe the current buffer the same way.

```rust
use rust_extensions::SecureStringBuilder;

let mut builder = SecureStringBuilder::with_capacity(64); // one allocation, one place to wipe

builder.push_str("postgres://user:");
builder.push_str("s3cr3t");
builder.push('@');
builder.push_line("localhost:5432"); // appends '\n'
builder.push_bytes(b"#").unwrap(); // UTF-8 checked

assert_eq!(builder.as_str(), "postgres://user:s3cr3t@localhost:5432\n#");
assert_eq!(builder.as_slice().len(), builder.len());
assert!(builder.capacity() >= 64);

builder.reserve(128); // grows now, wiping the old buffer

builder.clear(); // wipes, keeps the capacity
assert!(builder.is_empty());

// Debug shows the shape, never the content
assert!(format!("{:?}", builder).contains("len"));
// Dropping zeroes the allocation before freeing it.
```

- **Nothing owned comes out.** The content is lent through `as_str()` / `as_slice()` only. There is no `into_string()`, no `Clone` and no `Display` — each would put a copy into a `String` nobody wipes.
- **Pre-size when you can.** With `with_capacity(n)` the secret is written to one address only.
- **What it is not:** protection from someone who can read the process while the value is alive, or a guarantee that pages are never swapped. It stops a secret from outliving its use. Keep the builder short-lived.
