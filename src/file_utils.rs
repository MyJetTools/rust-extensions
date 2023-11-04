use crate::StrOrString;

pub fn format_path<'s>(src: impl Into<StrOrString<'s>>) -> StrOrString<'s> {
    let src: StrOrString<'s> = src.into();
    if !src.as_str().contains('~') {
        return src;
    }

    let path = std::env::var("HOME");

    if path.is_err() {
        return src;
    }

    let result = src.as_str().replace('~', path.unwrap().as_str());
    StrOrString::create_as_string(result)
}
