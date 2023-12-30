pub fn compare_strings_case_insensitive(src: &str, dst: &str) -> bool {
    if src.len() != dst.len() {
        return false;
    }

    for (src_char, dst_char) in src.chars().zip(dst.chars()) {
        if !src_char.eq_ignore_ascii_case(&dst_char) {
            return false;
        }
    }

    true
}

pub fn starts_with_case_insensitive(src: &str, start_with_str: &str) -> bool {
    if src.len() < start_with_str.len() {
        return false;
    }

    let mut i = 0;

    let src = src.as_bytes();

    for c in start_with_str.as_bytes() {
        if !src[i].eq_ignore_ascii_case(c) {
            return false;
        }
        i += 1;
    }

    true
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_different_cases() {
        assert!(super::compare_strings_case_insensitive("Hello", "hello"));
        assert!(super::compare_strings_case_insensitive("hello", "hello"));

        assert_eq!(
            false,
            super::compare_strings_case_insensitive("Yes", "hello")
        );

        assert_eq!(false, super::compare_strings_case_insensitive("Yes", "yey"));
    }

    #[test]
    fn test_starts_with() {
        assert!(super::starts_with_case_insensitive("/my/path", "/my/"));

        assert_eq!(
            false,
            super::starts_with_case_insensitive("/my/path", "my/")
        );

        assert_eq!(
            false,
            super::starts_with_case_insensitive("/my/path", "my/path1")
        );

        assert_eq!(true, super::starts_with_case_insensitive("/my/path", "/"));
    }
}
