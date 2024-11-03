use crate::{str_utils::StrUtils, ShortString};

#[derive(Debug, Clone, Copy)]
pub enum Scheme {
    Http,
    Https,
}

impl Scheme {
    pub fn try_parse(src: &str) -> Option<Self> {
        if src.starts_with_case_insensitive("http") {
            Some(Self::Http)
        } else if src.starts_with_case_insensitive("https") {
            Some(Self::Https)
        } else {
            None
        }
    }

    pub fn is_http(&self) -> bool {
        matches!(self, Self::Http)
    }

    pub fn is_https(&self) -> bool {
        matches!(self, Self::Https)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RemoteEndpoint<'s> {
    pub scheme: Option<Scheme>,
    pub host_str: &'s str,
    pub host_position: usize,
    pub port_position: Option<usize>,
}

impl<'s> RemoteEndpoint<'s> {
    pub fn try_parse(src: &'s str) -> Result<Self, String> {
        let mut first_separator = None;
        let mut second_separator = None;

        let mut pos = 0;
        for c in src.chars() {
            if c != ':' {
                pos += 1;
                continue;
            }

            if first_separator.is_none() {
                first_separator = Some(pos);
                pos += 1;
                continue;
            }
            second_separator = Some(pos);
            break;
        }

        if first_separator.is_none() {
            return Ok(Self {
                scheme: None,
                host_str: src,
                host_position: 0,
                port_position: None,
            });
        }

        let first_separator = first_separator.unwrap();

        let host_position = first_separator + 3;

        if let Some(scheme) = Scheme::try_parse(&src[..first_separator]) {
            match second_separator {
                Some(second_separator) => {
                    return Ok(Self {
                        scheme: Some(scheme),
                        host_str: src,
                        host_position,
                        port_position: Some(second_separator),
                    });
                }
                None => {
                    return Ok(Self {
                        scheme: Some(scheme),
                        host_str: src,
                        host_position,
                        port_position: None,
                    });
                }
            }
        } else {
            match second_separator {
                Some(_) => return Err(format!("Invalid remote_host endpoint string {src}")),
                None => {
                    return Ok(Self {
                        scheme: None,
                        host_str: src,
                        host_position: 0,
                        port_position: Some(first_separator),
                    });
                }
            }
        }
    }

    pub fn get_host(&self) -> &str {
        if let Some(port_position) = self.port_position {
            &self.host_str[self.host_position..port_position]
        } else {
            &self.host_str[self.host_position..]
        }
    }

    pub fn get_port_str(&self) -> Option<&str> {
        if let Some(port_position) = self.port_position {
            Some(&self.host_str[port_position + 1..])
        } else {
            None
        }
    }

    pub fn get_port(&self) -> Option<u16> {
        let port_str = self.get_port_str()?;

        match port_str.parse() {
            Ok(port) => Some(port),
            Err(_) => panic!("Invalid port string {port_str}"),
        }
    }

    pub fn get_host_port(&self, default_port: Option<u64>) -> ShortString {
        let mut result = ShortString::new_empty();

        result.push_str(&self.host_str[self.host_position..]);

        if self.port_position.is_some() {
            return result;
        }

        if let Some(default_port) = default_port {
            result.push_str(":");
            result.push_str(default_port.to_string().as_str());
        }
        result
    }

    pub fn as_str(&self) -> &str {
        self.host_str
    }
}

#[cfg(test)]
mod test {
    use super::RemoteEndpoint;

    #[test]
    fn test_http_with_port() {
        let result = RemoteEndpoint::try_parse("http://localhost:8000").unwrap();

        assert!(result.scheme.unwrap().is_http());
        assert_eq!(result.get_host(), "localhost");
        assert_eq!(result.get_port_str(), Some("8000"));
    }

    #[test]
    fn test_http_with_no_port() {
        let result = RemoteEndpoint::try_parse("http://localhost").unwrap();

        assert!(result.scheme.unwrap().is_http());
        assert_eq!(result.get_host(), "localhost");
        assert_eq!(result.get_port_str(), None);
    }

    #[test]
    fn test_no_scheme_but_has_port() {
        let result = RemoteEndpoint::try_parse("localhost:8888").unwrap();

        assert!(result.scheme.is_none());
        assert_eq!(result.get_host(), "localhost");
        assert_eq!(result.get_port_str(), Some("8888"));
    }

    #[test]
    fn test_no_scheme_and_no_port() {
        let result = RemoteEndpoint::try_parse("localhost").unwrap();

        assert!(result.scheme.is_none());
        assert_eq!(result.get_host(), "localhost");
        assert_eq!(result.get_port_str(), None);
    }

    #[test]
    fn test_get_host_port_with_default_port() {
        let result = RemoteEndpoint::try_parse("localhost").unwrap();

        let host_port = result.get_host_port(Some(80));
        assert_eq!(host_port.as_str(), "localhost:80");
    }
}
