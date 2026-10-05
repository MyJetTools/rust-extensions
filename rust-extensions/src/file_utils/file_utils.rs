use crate::StrOrString;

/// Expands a leading `~` into `$HOME`. A `~` anywhere else is a part of a name
/// (`report.txt~`) and is left as it is.
pub fn format_path<'s>(src: impl Into<StrOrString<'s>>) -> StrOrString<'s> {
    let src: StrOrString<'s> = src.into();

    let is_home = match src.as_str().strip_prefix('~') {
        Some(rest) => rest.is_empty() || rest.starts_with('/') || rest.starts_with('\\'),
        None => false,
    };

    if !is_home {
        return src;
    }

    let Ok(home) = std::env::var("HOME") else {
        return src;
    };

    StrOrString::create_as_string(format!("{}{}", home, &src.as_str()[1..]))
}

#[cfg(test)]
mod tests {
    use super::format_path;

    #[test]
    fn a_leading_tilde_is_the_home_directory() {
        let home = std::env::var("HOME").unwrap();

        assert_eq!(format_path("~/data").as_str(), format!("{}/data", home));
        assert_eq!(format_path("~").as_str(), home);
    }

    #[test]
    fn a_tilde_inside_a_name_is_kept() {
        assert_eq!(format_path("/data/report.txt~").as_str(), "/data/report.txt~");
        assert_eq!(format_path("/data/~backup").as_str(), "/data/~backup");
        assert_eq!(format_path("~user/data").as_str(), "~user/data");
    }
}
