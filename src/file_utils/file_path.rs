#[derive(Debug, Clone)]
pub struct FilePath {
    path: String,
}

impl FilePath {
    pub fn from_str(src: &str) -> Self {
        let path = super::file_utils::format_path(src);

        Self {
            path: path.to_string(),
        }
    }

    pub fn new_home() -> Self {
        let path = std::env::var("HOME").unwrap();
        Self { path }
    }

    #[cfg(test)]
    pub fn new_test_home(home_path: &str) -> Self {
        Self {
            path: home_path.to_string(),
        }
    }

    pub fn append_segment(&mut self, segment: &str) {
        let segment = if segment.ends_with(std::path::MAIN_SEPARATOR) {
            &segment[..segment.len() - 1]
        } else {
            segment
        };

        if !self.path.ends_with(std::path::MAIN_SEPARATOR) {
            self.path.push(std::path::MAIN_SEPARATOR);
        }

        if segment.starts_with(std::path::MAIN_SEPARATOR) {
            self.path.push_str(segment[1..].as_ref());
        } else {
            self.path.push_str(segment);
        }
    }

    pub fn remove_segment(&mut self) -> Option<String> {
        if self.path.len() <= 1 {
            return None;
        }

        let index = self.path.rfind(std::path::MAIN_SEPARATOR)?;

        let result = self.path.split_off(index + 1);

        if index == 0 {
            self.path.truncate(1);
        } else {
            self.path.truncate(index);
        }

        Some(result)
    }

    pub fn as_str(&self) -> &str {
        self.path.as_str()
    }

    pub fn to_string(&self) -> String {
        self.path.clone()
    }

    pub fn into_string(self) -> String {
        self.path
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test() {
        let mut path = super::FilePath::new_test_home("/root");

        path.append_segment("next");

        assert_eq!(path.as_str(), "/root/next");
    }

    #[test]
    fn test_2() {
        let mut path = super::FilePath::new_test_home("/root");

        path.append_segment("/next");

        assert_eq!(path.as_str(), "/root/next");

        let removed_segment = path.remove_segment().unwrap();
        assert_eq!(removed_segment, "next");
        assert_eq!(path.as_str(), "/root");
    }

    #[test]
    fn test_remote_segment() {
        let mut path = super::FilePath::new_test_home("/root");

        let removed_segment = path.remove_segment().unwrap();

        assert_eq!(removed_segment, "root");
        assert_eq!(path.as_str(), "/");

        let removed_segment = path.remove_segment();
        assert_eq!(removed_segment, None);
    }
}
