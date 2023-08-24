use crate::StrOrString;

pub fn format_path<'s>(src: &'s str) -> StrOrString<'s> {
    if !src.contains('~') {
        return StrOrString::create_as_str(src);
    }

    let path = std::env::var("HOME");

    if path.is_err() {
        return StrOrString::create_as_str(src);
    }

    let result = src.replace('~', path.unwrap().as_str());
    StrOrString::create_as_string(result)
}
