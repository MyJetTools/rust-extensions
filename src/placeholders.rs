use crate::slice_of_u8_utils::SliceOfU8Ext;

pub enum ContentToken<'s> {
    Text(&'s str),
    Placeholder(&'s str),
}

#[cfg(test)]
impl<'s> ContentToken<'s> {
    pub fn unwrap_as_text(&'s self) -> &'s str {
        match self {
            ContentToken::Text(value) => value,
            _ => panic!("ContentToken is not a text"),
        }
    }

    pub fn unwrap_as_placeholder(&'s self) -> &'s str {
        match self {
            ContentToken::Placeholder(value) => value,
            _ => panic!("ContentToken is not a placeholder"),
        }
    }
}

pub fn get_tokens_with_placeholders<'s>(content: &'s str) -> Vec<ContentToken<'s>> {
    let mut result = Vec::new();

    let content = content.as_bytes();

    const KEY_START: &str = "${";

    let mut i = 0;

    loop {
        let key_start = content.find_sequence_pos(KEY_START.as_bytes(), i);

        if key_start.is_none() {
            if i < content.len() {
                let text = std::str::from_utf8(&content[i..content.len()]).unwrap();
                result.push(ContentToken::Text(text));
            }

            break;
        }
        let key_start = key_start.unwrap();

        if i < key_start {
            let text = std::str::from_utf8(&content[i..key_start]).unwrap();
            result.push(ContentToken::Text(text));
        }

        let key_end = content.find_byte_pos(b'}', key_start);

        if key_end.is_none() {
            break;
        }

        let key_end = key_end.unwrap();

        let text = std::str::from_utf8(&content[key_start + 2..key_end]).unwrap();
        result.push(ContentToken::Placeholder(text));

        i = key_end + 1;
    }

    result
}

pub fn has_placeholder(src: &str, place_holder: &str) -> bool {
    for token in get_tokens_with_placeholders(src) {
        if let ContentToken::Placeholder(value) = token {
            if value == place_holder {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod test {

    #[test]
    fn get_tokens_with_placeholders() {
        let src = "my secret is ${secret1} and ${secret2} is my secret";

        let tokens = super::get_tokens_with_placeholders(src);

        assert_eq!(tokens.len(), 5);

        assert_eq!(tokens.get(0).unwrap().unwrap_as_text(), "my secret is ");
        assert_eq!(tokens.get(1).unwrap().unwrap_as_placeholder(), "secret1");
        assert_eq!(tokens.get(2).unwrap().unwrap_as_text(), " and ");
        assert_eq!(tokens.get(3).unwrap().unwrap_as_placeholder(), "secret2");
        assert_eq!(tokens.get(4).unwrap().unwrap_as_text(), " is my secret");
    }

    #[test]
    fn get_tokens_with_placeholder_at_the_end() {
        let src = "my secret is ${secret1} and ${secret2}";

        let tokens = super::get_tokens_with_placeholders(src);

        assert_eq!(tokens.len(), 4);

        assert_eq!(tokens.get(0).unwrap().unwrap_as_text(), "my secret is ");
        assert_eq!(tokens.get(1).unwrap().unwrap_as_placeholder(), "secret1");
        assert_eq!(tokens.get(2).unwrap().unwrap_as_text(), " and ");
        assert_eq!(tokens.get(3).unwrap().unwrap_as_placeholder(), "secret2");
    }

    #[test]
    fn get_tokens_with_placeholders_with_placeholder_at_beginning() {
        let src = "${secret1} and ${secret2} is my secret";

        let tokens = super::get_tokens_with_placeholders(src);

        assert_eq!(tokens.len(), 4);

        assert_eq!(tokens.get(0).unwrap().unwrap_as_placeholder(), "secret1");
        assert_eq!(tokens.get(1).unwrap().unwrap_as_text(), " and ");
        assert_eq!(tokens.get(2).unwrap().unwrap_as_placeholder(), "secret2");
        assert_eq!(tokens.get(3).unwrap().unwrap_as_text(), " is my secret");
    }
}
