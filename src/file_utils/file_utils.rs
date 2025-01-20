use crate::{ShortString, StrOrString};

pub fn format_path<'s>(src: impl Into<StrOrString<'s>>) -> StrOrString<'s> {
    let src: StrOrString<'s> = src.into();
    if !src.as_str().contains('~') {
        return src;
    }

    let path = std::env::var("HOME");

    if path.is_err() {
        return src;
    }

    let path = path.unwrap();

    if let Some(mut path_as_short_string) = ShortString::from_str(src.as_str()) {
        if path_as_short_string.replace("~", path.as_str()) {
            return StrOrString::create_as_short_string(path_as_short_string);
        }
    }

    StrOrString::create_as_string(src.as_str().replace("~", path.as_str()))
}
