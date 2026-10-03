/// Allocates a `Vec<u8>` of `len` bytes WITHOUT initializing them — a read buffer for a socket,
/// which is going to be overwritten by the read anyway, so zeroing it first is wasted work.
///
/// ```
/// use rust_extensions::vec_uninit;
///
/// let buf = vec_uninit![1024 * 1024];
/// assert_eq!(buf.len(), 1024 * 1024);
/// ```
///
/// The bytes hold garbage: only the part the read reported as written (`&buf[..n]`) may be looked at.
/// The element type is fixed to `u8` on purpose — garbage is a valid `u8`, it is not a valid `String`.
///
/// The `clippy::uninit_vec` lint is allowed inside the expansion, so the call site stays clean.
#[macro_export]
macro_rules! vec_uninit {
    ($len:expr) => {{
        let len: usize = $len;

        #[allow(clippy::uninit_vec)]
        let buf = {
            let mut buf: ::std::vec::Vec<u8> = ::std::vec::Vec::with_capacity(len);
            // SAFETY: `with_capacity(len)` gives room for at least `len` bytes, and the caller
            // writes them before reading - see the docs of the macro.
            unsafe {
                buf.set_len(len);
            }
            buf
        };

        buf
    }};
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_len_and_capacity() {
        let buf = vec_uninit![1024 * 1024];

        assert_eq!(buf.len(), 1024 * 1024);
        assert!(buf.capacity() >= 1024 * 1024);
    }

    #[test]
    fn test_zero_len() {
        let buf = vec_uninit![0];

        assert!(buf.is_empty());
    }

    #[test]
    fn test_buffer_is_writable() {
        let mut buf = vec_uninit![4];

        buf.copy_from_slice(&[1, 2, 3, 4]);

        assert_eq!(buf, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_len_expression_is_evaluated_once() {
        let mut calls = 0;
        let mut len = || {
            calls += 1;
            16
        };

        let buf = vec_uninit![len()];

        assert_eq!(buf.len(), 16);
        assert_eq!(calls, 1);
    }
}
