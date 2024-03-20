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

pub struct PlaceholdersIterator<'s> {
    content: &'s [u8],
    i: usize,
    reading_key: bool,
    open_token: &'static str,
    close_token: &'static str,
}

impl<'s> PlaceholdersIterator<'s> {
    pub fn new(content: &'s str, open_token: &'static str, close_token: &'static str) -> Self {
        Self {
            content: content.as_bytes(),
            i: 0,
            reading_key: false,
            open_token,
            close_token,
        }
    }
}

impl<'s> Iterator for PlaceholdersIterator<'s> {
    type Item = ContentToken<'s>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.content.len() {
            return None;
        }

        if !self.reading_key {
            let key_start = self
                .content
                .find_sequence_pos(self.open_token.as_bytes(), self.i);

            if key_start.is_none() {
                let text = std::str::from_utf8(&self.content[self.i..]).unwrap();
                self.i = self.content.len();
                return Some(ContentToken::Text(text));
            }

            let key_start = key_start.unwrap();

            if self.i < key_start {
                let text = std::str::from_utf8(&self.content[self.i..key_start]).unwrap();
                self.i = key_start;
                self.reading_key = true;
                return Some(ContentToken::Text(text));
            } else {
                self.reading_key = true;
                self.i = key_start;
            }
        }

        let key_end = self
            .content
            .find_sequence_pos(self.close_token.as_bytes(), self.i);

        if key_end.is_none() {
            let text = std::str::from_utf8(&self.content[self.i..]).unwrap();
            self.i = self.content.len();
            return Some(ContentToken::Text(text));
        }

        let key_end = key_end.unwrap();

        let text =
            std::str::from_utf8(&self.content[self.i + self.open_token.len()..key_end]).unwrap();
        self.i = key_end + self.close_token.len();
        self.reading_key = false;
        Some(ContentToken::Placeholder(text))
    }
}

pub fn has_placeholder(
    src: &str,
    place_holder: &str,
    open_token: &'static str,
    close_token: &'static str,
) -> bool {
    for token in PlaceholdersIterator::new(src, open_token, close_token) {
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
    use crate::placeholders::PlaceholdersIterator;

    #[test]
    fn get_tokens_with_placeholders() {
        let src = "my secret is ${secret1} and ${secret2} is my secret";

        let tokens: Vec<_> = PlaceholdersIterator::new(src, "${", "}").collect();

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

        let tokens: Vec<_> = PlaceholdersIterator::new(src, "${", "}").collect();

        assert_eq!(tokens.len(), 4);

        assert_eq!(tokens.get(0).unwrap().unwrap_as_text(), "my secret is ");
        assert_eq!(tokens.get(1).unwrap().unwrap_as_placeholder(), "secret1");
        assert_eq!(tokens.get(2).unwrap().unwrap_as_text(), " and ");
        assert_eq!(tokens.get(3).unwrap().unwrap_as_placeholder(), "secret2");
    }

    #[test]
    fn get_tokens_with_placeholders_with_placeholder_at_beginning() {
        let src = "${secret1} and ${secret2} is my secret";

        let tokens: Vec<_> = PlaceholdersIterator::new(src, "${", "}").collect();

        assert_eq!(tokens.len(), 4);

        assert_eq!(tokens.get(0).unwrap().unwrap_as_placeholder(), "secret1");
        assert_eq!(tokens.get(1).unwrap().unwrap_as_text(), " and ");
        assert_eq!(tokens.get(2).unwrap().unwrap_as_placeholder(), "secret2");
        assert_eq!(tokens.get(3).unwrap().unwrap_as_text(), " is my secret");
    }
}
