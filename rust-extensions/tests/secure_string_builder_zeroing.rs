//! End-to-end proof that `SecureStringBuilder` hands every allocation back to the
//! allocator zeroed.
//!
//! The unit tests inside the module inspect a buffer that is still alive, which
//! shows the wipe happened but not that it happened *before the free*. This test
//! answers that directly: it installs a global allocator which looks at the block
//! contents inside `dealloc`, while the memory is still valid to read, and counts
//! how many freed blocks were all zeroes and how many were not.
//!
//! It lives in its own integration test because a `#[global_allocator]` applies to
//! a whole binary.
//!
//! Worth running in release too — `cargo test --release --test
//! secure_string_builder_zeroing` — since the whole point of the volatile writes
//! is that an optimiser may not drop them.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

use rust_extensions::SecureStringBuilder;

thread_local! {
    /// Tracking is per-thread so the counters cannot be polluted by whatever the
    /// test harness frees on its own threads. `const`-initialised, so reading it
    /// from inside the allocator never triggers a lazy init (which would
    /// allocate).
    static TRACKING: Cell<bool> = const { Cell::new(false) };
}

static CLEAN_FREES: AtomicUsize = AtomicUsize::new(0);
static DIRTY_FREES: AtomicUsize = AtomicUsize::new(0);

struct TrackingAllocator;

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let tracking = TRACKING.try_with(|it| it.get()).unwrap_or(false);

        // `align() == 1` keeps this to byte buffers - which is what a `String`
        // allocates - so no other kind of allocation can land in the counters.
        if tracking && layout.align() == 1 && layout.size() > 0 {
            let block = std::slice::from_raw_parts(ptr, layout.size());

            if block.iter().all(|byte| *byte == 0) {
                CLEAN_FREES.fetch_add(1, Ordering::SeqCst);
            } else {
                DIRTY_FREES.fetch_add(1, Ordering::SeqCst);
            }
        }

        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

/// Runs `body` with this thread's deallocations counted, and answers
/// `(clean, dirty)`. `body` must not allocate anything besides the buffer under
/// test, and must leave no byte-buffer with an uninitialised tail to be freed -
/// otherwise the hook above would read uninitialised memory.
fn count_frees(body: impl FnOnce()) -> (usize, usize) {
    CLEAN_FREES.store(0, Ordering::SeqCst);
    DIRTY_FREES.store(0, Ordering::SeqCst);

    TRACKING.with(|it| it.set(true));
    body();
    TRACKING.with(|it| it.set(false));

    (
        CLEAN_FREES.load(Ordering::SeqCst),
        DIRTY_FREES.load(Ordering::SeqCst),
    )
}

/// Both halves live in one test on purpose: the counters are global, and
/// `cargo test` would run two test functions on two threads at once.
#[test]
fn every_buffer_is_zeroed_before_it_is_freed() {
    // --- the type under test: grows several times, then is dropped ---------
    let (clean, dirty) = count_frees(|| {
        let mut builder = SecureStringBuilder::new();

        for _ in 0..50 {
            builder.push_str("s3cr3t-");
        }

        drop(builder);
    });

    assert_eq!(
        0, dirty,
        "a buffer still holding the secret was handed back to the allocator"
    );

    // 32 -> 64 -> 128 -> 256 -> 512 while filling 350 bytes: four retired
    // buffers plus the one released by `Drop`.
    assert_eq!(
        5, clean,
        "expected every retired buffer plus the final one to be freed zeroed"
    );

    // --- the control: a plain String, to prove the hook can see the difference
    let (_, dirty) = count_frees(|| {
        let mut plain = String::with_capacity(8);
        plain.push_str("s3cr3t!!"); // len == capacity: no uninitialised tail

        plain.push('X'); // grows - frees the 8-byte block as-is

        // Fill the new block up to its capacity as well, so the free on drop has
        // nothing uninitialised in it either.
        while plain.len() < plain.capacity() {
            plain.push('X');
        }

        drop(plain);
    });

    assert_eq!(
        2, dirty,
        "the control did not observe a non-zeroed free - the assertion above proves nothing"
    );
}
