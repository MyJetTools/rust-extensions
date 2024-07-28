pub trait StrUtils<'s> {
    fn to_str(&'s self) -> &str;

    fn eq_case_insensitive(&'s self, dst: &str) -> bool {
        compare_strings_case_insensitive(self.to_str(), dst)
    }

    fn starts_with_case_insensitive(&'s self, start_with_str: &str) -> bool {
        starts_with_case_insensitive(self.to_str(), start_with_str)
    }

    fn split_up_to_2_lines(&'s self, slipt_line: &str) -> Option<(&'s str, Option<&'s str>)> {
        let mut first = None;
        let mut second = None;

        for line in self.to_str().split(slipt_line) {
            if first.is_none() {
                first = Some(line);
                continue;
            }

            if second.is_none() {
                second = Some(line);
                continue;
            }

            return None;
        }

        let first = first?;

        Some((first, second))
    }

    fn split_exact_to_2_lines(&'s self, slipt_line: &str) -> Option<(&'s str, &'s str)> {
        let mut first = None;
        let mut second = None;

        for line in self.to_str().split(slipt_line) {
            if first.is_none() {
                first = Some(line);
                continue;
            }

            if second.is_none() {
                second = Some(line);
                continue;
            }

            return None;
        }

        let first = first?;
        let second = second?;

        Some((first, second))
    }

    fn split_exact_to_3_lines(&'s self, slipt_line: &str) -> Option<(&'s str, &'s str, &'s str)> {
        let mut first = None;
        let mut second = None;
        let mut third = None;

        for line in self.to_str().split(slipt_line) {
            if first.is_none() {
                first = Some(line);
                continue;
            }

            if second.is_none() {
                second = Some(line);
                continue;
            }

            if third.is_none() {
                third = Some(line);
                continue;
            }

            return None;
        }

        let first = first?;
        let second = second?;
        let third = third?;

        Some((first, second, third))
    }

    fn split_2_or_3_lines(
        &'s self,
        slipt_line: &str,
    ) -> Option<(&'s str, &'s str, Option<&'s str>)> {
        let mut first = None;
        let mut second = None;
        let mut third = None;

        for line in self.to_str().split(slipt_line) {
            if first.is_none() {
                first = Some(line);
                continue;
            }

            if second.is_none() {
                second = Some(line);
                continue;
            }

            if third.is_none() {
                third = Some(line);
                continue;
            }

            return None;
        }

        let first = first?;
        let second = second?;

        Some((first, second, third))
    }
}

impl<'s> StrUtils<'s> for &'s str {
    fn to_str(&'s self) -> &'s str {
        self
    }
}

impl<'s> StrUtils<'s> for &'s String {
    fn to_str(&'s self) -> &'s str {
        self
    }
}

impl<'s> StrUtils<'s> for String {
    fn to_str(&'s self) -> &str {
        self
    }
}

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
