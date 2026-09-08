use std::sync::atomic::{compiler_fence, Ordering};

/// Capacity the buffer jumps to the first time it has to allocate, unless a
/// bigger one is asked for right away. Small enough not to waste a page on a
/// builder that ends up holding a short token, big enough that a password or a
/// connection string is assembled without a single re-allocation.
const FIRST_CAPACITY: usize = 32;

/// A [`String`] builder for data that must not be left behind in freed memory —
/// passwords, tokens, connection strings, private keys.
///
/// # What it does differently
///
/// A plain `String` grows by handing the old block back to the allocator. The
/// bytes are still there: `realloc` copies them into the new block and frees the
/// old one **as-is**, so a secret that grew from 8 to 16 to 32 bytes leaves three
/// copies of itself in the heap's free lists, and dropping the string frees the
/// last one just as untouched. Anything that later reads that memory — the next
/// allocation, a core dump, a swapped-out page, a heap-scanning exploit — reads
/// the secret.
///
/// `SecureStringBuilder` never lets the `String` re-allocate itself. It watches
/// every push, and the moment the next one would not fit it:
///
/// 1. allocates a new buffer of the grown capacity,
/// 2. copies the current content into it,
/// 3. **overwrites the whole old allocation with zeroes** — every byte of its
///    `capacity()`, not only the `len()` that was in use,
/// 4. and only then frees it.
///
/// [`Drop`] does the same to the final buffer, and so does
/// [`clear`](SecureStringBuilder::clear).
///
/// The zeroing is done with [`std::ptr::write_volatile`] byte by byte, followed
/// by a [`compiler_fence`]. A plain `memset` right before a free is dead code and
/// the optimiser is entitled to delete it; a volatile write is an observable side
/// effect and may not be touched. That is the whole reason this type exists — a
/// `.fill(0)` that gets optimised away looks exactly like one that works.
///
/// # Nothing owned comes out
///
/// Because the guarantee is enforced by `Drop`, the content is only ever handed
/// out **by reference** — [`as_str`](SecureStringBuilder::as_str) and
/// [`as_slice`](SecureStringBuilder::as_slice). There is deliberately no
/// `into_string()`, no `Clone`, and no `Display` (which would make `.to_string()`
/// available): every one of those would put a copy of the secret into a `String`
/// nobody wipes, which is the exact problem this type is here to solve. Use the
/// borrow at the point where the secret is needed, and let the builder die.
///
/// `Debug` prints the shape only — never the content.
///
/// # What it does not protect against
///
/// This is a best-effort defence against the secret **outliving its use** in
/// memory that is no longer owned. It cannot stop the OS from swapping a live
/// page out, it does not lock pages into RAM, and it is no defence at all against
/// someone who can read the process while the value is alive. Keep the builder
/// short-lived.
///
/// # Example
///
/// ```
/// use rust_extensions::SecureStringBuilder;
///
/// let mut builder = SecureStringBuilder::new();
///
/// builder.push_str("postgres://user:");
/// builder.push_str("s3cr3t");
/// builder.push('@');
/// builder.push_str("localhost:5432");
///
/// assert_eq!("postgres://user:s3cr3t@localhost:5432", builder.as_str());
///
/// // ... connect(builder.as_str()) ...
///
/// // Dropping here zeroes the whole allocation before freeing it.
/// ```
///
/// Pre-sizing avoids growth altogether, so the secret is only ever written to one
/// address:
///
/// ```
/// use rust_extensions::SecureStringBuilder;
///
/// let mut builder = SecureStringBuilder::with_capacity(64);
/// builder.push_str("a-long-lived-token");
/// assert_eq!(64, builder.capacity());
/// ```
pub struct SecureStringBuilder {
    buffer: String,
}

impl SecureStringBuilder {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Allocates room for `capacity` bytes up front.
    ///
    /// Worth using whenever the final size is known: a builder which never has to
    /// grow never has to copy the secret to a second address, so there is exactly
    /// one allocation to wipe instead of a chain of them.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: String::with_capacity(capacity),
        }
    }

    /// The content built so far.
    ///
    /// This — and [`as_slice`](SecureStringBuilder::as_slice) — is the only way
    /// the content leaves the builder. The borrow keeps it tied to the builder's
    /// lifetime, which is what makes the wipe on `Drop` mean anything.
    pub fn as_str(&self) -> &str {
        self.buffer.as_str()
    }

    /// The content built so far, as raw bytes (always valid UTF-8).
    pub fn as_slice(&self) -> &[u8] {
        self.buffer.as_bytes()
    }

    /// Bytes written so far.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// How many bytes fit before the buffer has to be grown (and the current
    /// allocation wiped).
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Makes room for `additional` more bytes, growing — and therefore wiping the
    /// old allocation — at most once.
    pub fn reserve(&mut self, additional: usize) {
        self.grow_to_fit(additional);
    }

    pub fn push_str(&mut self, src: &str) {
        self.write(src);
    }

    /// Appends `src` followed by `'\n'`.
    pub fn push_line(&mut self, src: &str) {
        self.grow_to_fit(src.len().saturating_add(1));
        self.write(src);
        self.write("\n");
    }

    pub fn push(&mut self, c: char) {
        let mut encode_buffer = [0u8; 4];
        let encoded = c.encode_utf8(&mut encode_buffer);
        self.write(encoded);

        // The char has been copied into `encode_buffer` on the way in - it is a
        // stack copy of a piece of the secret, so it gets the same treatment as a
        // heap one.
        wipe_slice(&mut encode_buffer);
    }

    /// Appends `src`, which has to be valid UTF-8.
    pub fn push_bytes(&mut self, src: &[u8]) -> Result<(), std::str::Utf8Error> {
        let as_str = std::str::from_utf8(src)?;
        self.write(as_str);
        Ok(())
    }

    /// Zeroes the whole allocation and empties the builder, **keeping** the
    /// capacity — so re-filling it does not allocate again.
    pub fn clear(&mut self) {
        wipe_string(&mut self.buffer);
    }

    fn write(&mut self, src: &str) {
        if src.is_empty() {
            return;
        }

        self.grow_to_fit(src.len());

        let capacity_before = self.buffer.capacity();
        self.buffer.push_str(src);

        // `String::push_str` goes through `Vec::reserve`, which is documented to
        // do nothing when the capacity is already sufficient - and `grow_to_fit`
        // has just made sure it is. Pinned here because the entire guarantee of
        // this type rests on it: a re-allocation happening in there would free the
        // old block behind our back, unwiped.
        debug_assert_eq!(
            capacity_before,
            self.buffer.capacity(),
            "SecureStringBuilder: the inner String re-allocated on its own - the old buffer was freed without being wiped"
        );
    }

    fn grow_to_fit(&mut self, additional: usize) {
        let required = self.buffer.len().saturating_add(additional);

        if required <= self.buffer.capacity() {
            return;
        }

        let wiped_old_buffer = self.reallocate(next_capacity(self.buffer.capacity(), required));
        drop(wiped_old_buffer);
    }

    /// Moves the content into a fresh buffer of `new_capacity` bytes, wipes the
    /// old allocation and hands it back so the caller can drop it.
    ///
    /// It is returned rather than dropped in place so that the wipe itself is
    /// observable to a test - the returned `String` still owns the allocation the
    /// zeroes were written into.
    #[must_use]
    fn reallocate(&mut self, new_capacity: usize) -> String {
        debug_assert!(new_capacity >= self.buffer.len());

        let mut new_buffer = String::with_capacity(new_capacity);
        // Fits by construction, so this cannot re-allocate either.
        new_buffer.push_str(self.buffer.as_str());

        let mut old_buffer = std::mem::replace(&mut self.buffer, new_buffer);
        wipe_string(&mut old_buffer);
        old_buffer
    }
}

/// Doubling growth, floored at [`FIRST_CAPACITY`] and never below what is
/// actually needed.
///
/// Doubling is not only about amortised cost here: every growth is one more
/// address the secret has been written to, so the fewer of them the better.
fn next_capacity(current: usize, required: usize) -> usize {
    let grown = if current == 0 {
        FIRST_CAPACITY
    } else {
        current.saturating_mul(2)
    };

    if grown < required {
        required
    } else {
        grown
    }
}

/// Overwrites every byte the string's allocation owns - its whole `capacity()`,
/// not just the `len()` in use - with zeroes, and empties it.
///
/// The capacity is what matters: bytes past `len` hold whatever an earlier,
/// longer content left there, and they are freed along with the rest.
fn wipe_string(buffer: &mut String) {
    let capacity = buffer.capacity();

    if capacity == 0 {
        // Nothing was ever allocated - the pointer is dangling, and writing
        // through it would be undefined behaviour.
        return;
    }

    // SAFETY: the borrow ends with the string empty, which is valid UTF-8. In
    // between, every byte of the allocation is written to (so all of it is
    // initialised), and nothing is read.
    let as_vec = unsafe { buffer.as_mut_vec() };

    let ptr = as_vec.as_mut_ptr();

    for offset in 0..capacity {
        // SAFETY: `offset < capacity`, so the pointer stays inside the allocation
        // the vector owns, and `u8` needs no alignment.
        unsafe {
            std::ptr::write_volatile(ptr.add(offset), 0);
        }
    }

    compiler_fence(Ordering::SeqCst);

    // SAFETY: 0 <= capacity, and an empty string is trivially valid UTF-8.
    unsafe {
        as_vec.set_len(0);
    }
}

/// The same treatment for a plain byte buffer — used for the stack scratch space
/// a `char` is encoded through on its way into the builder.
fn wipe_slice(slice: &mut [u8]) {
    let ptr = slice.as_mut_ptr();

    for offset in 0..slice.len() {
        // SAFETY: `offset < slice.len()`, so the pointer stays inside the slice.
        unsafe {
            std::ptr::write_volatile(ptr.add(offset), 0);
        }
    }

    compiler_fence(Ordering::SeqCst);
}

impl Drop for SecureStringBuilder {
    fn drop(&mut self) {
        wipe_string(&mut self.buffer);
    }
}

impl Default for SecureStringBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Shape only — the content of a secure builder has no business in a log line.
impl std::fmt::Debug for SecureStringBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecureStringBuilder")
            .field("len", &self.buffer.len())
            .field("capacity", &self.buffer.capacity())
            .finish()
    }
}

impl crate::AsStr for SecureStringBuilder {
    fn as_str(&self) -> &str {
        self.buffer.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads the whole allocation of a string back, including the bytes past
    /// `len`. Sound only right after [`wipe_string`], which is what makes every
    /// byte of the capacity initialised.
    ///
    /// The pointer has to come from the `Vec`: `String::as_ptr` goes through
    /// `Deref<Target = str>`, so its provenance spans only `len` bytes - and
    /// after a wipe that is zero of them. Reading `capacity` bytes through such a
    /// pointer is undefined behaviour under Stacked Borrows, which `cargo miri
    /// test` rejects.
    fn read_whole_allocation(src: &mut String) -> &[u8] {
        // SAFETY: the bytes are only read, never interpreted as UTF-8, and the
        // whole capacity is initialised by the wipe that must precede this call.
        let as_vec = unsafe { src.as_mut_vec() };
        let capacity = as_vec.capacity();
        unsafe { std::slice::from_raw_parts(as_vec.as_ptr(), capacity) }
    }

    #[test]
    fn builds_the_string() {
        let mut builder = SecureStringBuilder::new();

        builder.push_str("Hello");
        builder.push(' ');
        builder.push_str("world");
        builder.push('!');

        assert_eq!("Hello world!", builder.as_str());
        assert_eq!(b"Hello world!", builder.as_slice());
        assert_eq!(12, builder.len());
    }

    #[test]
    fn content_survives_growth() {
        let mut builder = SecureStringBuilder::with_capacity(4);

        let mut expected = String::new();

        for i in 0..1000 {
            let next = format!("{}-", i);
            builder.push_str(next.as_str());
            expected.push_str(next.as_str());
        }

        assert_eq!(expected.as_str(), builder.as_str());
    }

    #[test]
    fn growth_wipes_the_whole_old_allocation() {
        let mut builder = SecureStringBuilder::with_capacity(8);
        builder.push_str("s3cr3t");

        // Grow by hand so the buffer being retired can be inspected before it is
        // freed - which is exactly what `push_str` does internally.
        let mut old_buffer = builder.reallocate(64);

        assert_eq!(8, old_buffer.capacity());
        assert!(read_whole_allocation(&mut old_buffer)
            .iter()
            .all(|byte| *byte == 0));

        // ... and the content moved over intact.
        assert_eq!("s3cr3t", builder.as_str());
        assert_eq!(64, builder.capacity());
    }

    #[test]
    fn wipe_zeroes_past_the_length_too() {
        let mut buffer = String::with_capacity(32);
        buffer.push_str("a-long-secret-which-is-then-cut");
        buffer.truncate(3);

        wipe_string(&mut buffer);

        assert_eq!(0, buffer.len());
        assert_eq!(32, buffer.capacity());
        assert!(read_whole_allocation(&mut buffer).iter().all(|byte| *byte == 0));
    }

    #[test]
    fn wipe_of_a_never_allocated_string_is_a_no_op() {
        let mut buffer = String::new();
        wipe_string(&mut buffer);
        assert_eq!(0, buffer.capacity());
    }

    #[test]
    fn clear_wipes_and_keeps_the_capacity() {
        let mut builder = SecureStringBuilder::with_capacity(32);
        builder.push_str("s3cr3t");

        builder.clear();

        assert!(builder.is_empty());
        assert_eq!(32, builder.capacity());
        assert!(read_whole_allocation(&mut builder.buffer)
            .iter()
            .all(|byte| *byte == 0));

        builder.push_str("next");
        assert_eq!("next", builder.as_str());
    }

    #[test]
    fn pushing_never_lets_the_inner_string_reallocate() {
        let mut builder = SecureStringBuilder::new();

        for i in 0..500 {
            let capacity_before = builder.capacity();
            let expected_growth = builder.len() + 1 > capacity_before;

            builder.push((b'a' + (i % 26) as u8) as char);

            if !expected_growth {
                assert_eq!(capacity_before, builder.capacity());
            }
        }
    }

    #[test]
    fn capacity_grows_by_doubling() {
        assert_eq!(FIRST_CAPACITY, next_capacity(0, 1));
        assert_eq!(1000, next_capacity(0, 1000));
        assert_eq!(64, next_capacity(32, 33));
        assert_eq!(300, next_capacity(100, 300));
        assert_eq!(usize::MAX, next_capacity(usize::MAX, usize::MAX));
    }

    #[test]
    fn empty_push_does_not_allocate() {
        let mut builder = SecureStringBuilder::new();
        builder.push_str("");
        assert_eq!(0, builder.capacity());
        assert!(builder.is_empty());
    }

    #[test]
    fn reserve_grows_once() {
        let mut builder = SecureStringBuilder::new();
        builder.push_str("abc");

        builder.reserve(1000);
        let capacity = builder.capacity();
        assert!(capacity >= 1003);

        for _ in 0..1000 {
            builder.push('x');
        }

        assert_eq!(capacity, builder.capacity());
    }

    #[test]
    fn push_line_appends_the_new_line() {
        let mut builder = SecureStringBuilder::new();
        builder.push_line("first");
        builder.push_line("second");

        assert_eq!("first\nsecond\n", builder.as_str());
    }

    #[test]
    fn push_bytes_validates_utf8() {
        let mut builder = SecureStringBuilder::new();

        assert!(builder.push_bytes("ok-".as_bytes()).is_ok());
        assert!(builder.push_bytes(&[0xF0, 0x28]).is_err());

        // The rejected input left nothing behind.
        assert_eq!("ok-", builder.as_str());
    }

    #[test]
    fn multi_byte_chars_are_handled() {
        let mut builder = SecureStringBuilder::with_capacity(1);

        builder.push('€');
        builder.push('🔐');
        builder.push_str("café-naïve");

        assert_eq!("€🔐café-naïve", builder.as_str());
    }

    #[test]
    fn debug_does_not_leak_the_content() {
        let mut builder = SecureStringBuilder::with_capacity(32);
        builder.push_str("s3cr3t");

        let rendered = format!("{:?}", builder);

        assert!(!rendered.contains("s3cr3t"));
        assert!(rendered.contains("len: 6"));
    }
}
