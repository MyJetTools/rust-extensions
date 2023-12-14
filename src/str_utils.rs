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
}
