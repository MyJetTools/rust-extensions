use crate::ShortString;

#[derive(Debug, Clone, Copy)]
pub enum Scheme {
    Http,
    Https,
    Ws,
    Wss,
    UnixSocket,
}

impl Scheme {
    pub fn try_parse(src: &str) -> Option<Self> {
        if src.eq_ignore_ascii_case("https") {
            Some(Self::Https)
        } else if src.eq_ignore_ascii_case("http") {
            Some(Self::Http)
        } else if src.eq_ignore_ascii_case("ws") {
            Some(Self::Ws)
        } else if src.eq_ignore_ascii_case("wss") {
            Some(Self::Wss)
        } else if src.eq_ignore_ascii_case("http+unix") {
            Some(Self::UnixSocket)
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

    pub fn is_ws(&self) -> bool {
        matches!(self, Self::Ws)
    }

    pub fn is_wss(&self) -> bool {
        matches!(self, Self::Wss)
    }

    pub fn is_unix_socket(&self) -> bool {
        match self {
            Scheme::UnixSocket => true,
            _ => false,
        }
    }

    pub fn host_postfix_len(&self) -> usize {
        match self {
            Scheme::UnixSocket => 2,
            _ => 3,
        }
    }
}

enum ReadingEndpointMode {
    LookingForSchemeEnd,
    NextSymbolAfterSchemeEnd,
    ReadingSchemeLastSymbols,
    LookingForEndOfHost,
    ReadingPort,
}

#[derive(Debug, Clone, Copy)]
pub struct RemoteEndpointInner {
    scheme: Option<Scheme>,
    host_position: usize,
    host_end_position: usize,
    port_position: Option<usize>,
    http_path_and_query_position: Option<usize>,
}

impl RemoteEndpointInner {
    pub fn try_parse(src: &str) -> Result<Self, String> {
        let mut scheme_name_end_position = None;

        let mut host_end_position = None;
        let mut http_path_and_query_position = None;

        let mut port_position = None;

        let mut reading_mode = ReadingEndpointMode::LookingForSchemeEnd;

        for (pos, c) in src.chars().enumerate() {
            match reading_mode {
                ReadingEndpointMode::LookingForSchemeEnd => {
                    if c == ':' {
                        reading_mode = ReadingEndpointMode::NextSymbolAfterSchemeEnd;
                    }
                }
                ReadingEndpointMode::NextSymbolAfterSchemeEnd => {
                    if c.is_ascii_digit() {
                        reading_mode = ReadingEndpointMode::ReadingPort;
                        port_position = Some(pos - 1);
                        host_end_position = port_position;
                        continue;
                    }

                    if c == '/' {
                        scheme_name_end_position = Some(pos - 1);
                        reading_mode = ReadingEndpointMode::ReadingSchemeLastSymbols;
                        continue;
                    }

                    host_end_position = scheme_name_end_position;
                    scheme_name_end_position = None;
                    reading_mode = ReadingEndpointMode::LookingForEndOfHost
                }

                ReadingEndpointMode::ReadingSchemeLastSymbols => {
                    if c != '/' {
                        reading_mode = ReadingEndpointMode::LookingForEndOfHost;
                    }
                }

                ReadingEndpointMode::LookingForEndOfHost => match c {
                    ':' => {
                        host_end_position = Some(pos);
                        port_position = Some(pos);
                    }

                    '/' => {
                        host_end_position = Some(pos);
                        http_path_and_query_position = Some(pos);
                        break;
                    }
                    _ => {}
                },
                ReadingEndpointMode::ReadingPort => {
                    if !c.is_ascii_digit() {
                        http_path_and_query_position = Some(pos);
                        break;
                    }
                }
            }
        }

        if host_end_position.is_none() {
            host_end_position = Some(src.len());
        }

        if scheme_name_end_position.is_none() {
            return Ok(Self {
                scheme: None,
                host_position: 0,
                host_end_position: host_end_position.unwrap(),
                port_position,
                http_path_and_query_position,
            });
        }

        let scheme_name_end_position = scheme_name_end_position.unwrap();

        let scheme = &src[..scheme_name_end_position];

        if let Some(scheme) = Scheme::try_parse(scheme) {
            return Ok(Self {
                scheme: Some(scheme),
                host_position: scheme_name_end_position + scheme.host_postfix_len(),
                host_end_position: host_end_position.unwrap(),
                port_position,
                http_path_and_query_position,
            });
        }

        panic!("Invalid scheme name {}", scheme);
    }

    pub fn get_host<'s>(&self, src: &'s str) -> &'s str {
        if let Some(port_position) = self.port_position {
            return &src[self.host_position..port_position];
        }

        if let Some(path_and_query_position) = self.http_path_and_query_position {
            return &src[self.host_position..path_and_query_position];
        }
        &src[self.host_position..]
    }

    pub fn get_port_str<'s>(&self, src: &'s str) -> Option<&'s str> {
        if let Some(port_position) = self.port_position {
            if let Some(path_and_query_position) = self.http_path_and_query_position {
                Some(&src[port_position + 1..path_and_query_position])
            } else {
                Some(&src[port_position + 1..])
            }
        } else {
            None
        }
    }

    pub fn get_port(&self, src: &str) -> Option<u16> {
        let port_str = self.get_port_str(src)?;

        match port_str.parse() {
            Ok(port) => Some(port),
            Err(_) => panic!("Invalid port string {port_str}"),
        }
    }

    pub fn get_host_port(&self, src: &str, default_port: Option<u64>) -> ShortString {
        let mut result = ShortString::new_empty();

        result.push_str(&src[self.host_position..self.host_end_position]);

        if self.port_position.is_some() {
            return result;
        }

        if let Some(default_port) = default_port {
            result.push_str(":");
            result.push_str(default_port.to_string().as_str());
        }
        result
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RemoteEndpoint<'s> {
    host_str: &'s str,
    inner: RemoteEndpointInner,
}

impl<'s> RemoteEndpoint<'s> {
    pub fn try_parse(src: &'s str) -> Result<Self, String> {
        let inner = RemoteEndpointInner::try_parse(src)?;
        Ok(Self {
            host_str: src,
            inner,
        })
    }

    pub fn to_owned(&self) -> RemoteEndpointOwned {
        RemoteEndpointOwned {
            host_str: self.host_str.to_string(),
            inner: self.inner,
        }
    }

    pub fn get_scheme(&self) -> Option<Scheme> {
        self.inner.scheme
    }

    pub fn get_host(&self) -> &str {
        self.inner.get_host(self.host_str)
    }

    pub fn get_port_str(&self) -> Option<&str> {
        self.inner.get_port_str(self.host_str)
    }

    pub fn get_port(&self) -> Option<u16> {
        self.inner.get_port(self.host_str)
    }

    pub fn get_host_port(&self, default_port: Option<u64>) -> ShortString {
        self.inner.get_host_port(self.host_str, default_port)
    }

    pub fn get_http_path_and_query(&self) -> Option<&str> {
        let pos = self.inner.http_path_and_query_position?;
        Some(&self.host_str[pos..])
    }

    pub fn as_str(&self) -> &str {
        self.host_str
    }
}

#[derive(Debug, Clone)]
pub struct RemoteEndpointOwned {
    host_str: String,
    inner: RemoteEndpointInner,
}

impl RemoteEndpointOwned {
    pub fn try_parse(src: String) -> Result<Self, String> {
        let inner = RemoteEndpointInner::try_parse(&src)?;
        Ok(Self {
            host_str: src,
            inner,
        })
    }

    pub fn to_ref(&self) -> RemoteEndpoint {
        RemoteEndpoint {
            host_str: self.host_str.as_str(),
            inner: self.inner,
        }
    }

    pub fn get_scheme(&self) -> Option<Scheme> {
        self.inner.scheme
    }

    pub fn get_host(&self) -> &str {
        self.inner.get_host(&self.host_str)
    }

    pub fn get_port_str(&self) -> Option<&str> {
        self.inner.get_port_str(&self.host_str)
    }

    pub fn get_port(&self) -> Option<u16> {
        self.inner.get_port(&self.host_str)
    }

    pub fn get_host_port(&self, default_port: Option<u64>) -> ShortString {
        self.inner.get_host_port(&self.host_str, default_port)
    }

    pub fn as_str(&self) -> &str {
        &self.host_str
    }

    pub fn get_http_path_and_query(&self) -> Option<&str> {
        let pos = self.inner.http_path_and_query_position?;
        Some(&self.host_str[pos..])
    }
}

#[cfg(test)]
mod test {
    use super::RemoteEndpoint;

    #[test]
    fn test_http_with_port() {
        let result = RemoteEndpoint::try_parse("http://localhost:8000").unwrap();

        assert!(result.get_scheme().unwrap().is_http());
        assert_eq!(result.get_host(), "localhost");
        assert_eq!(result.get_port_str(), Some("8000"));
    }

    #[test]
    fn test_http_with_no_port() {
        let result = RemoteEndpoint::try_parse("http://localhost").unwrap();

        assert!(result.get_scheme().unwrap().is_http());
        assert_eq!(result.get_host(), "localhost");
        assert_eq!(result.get_port_str(), None);
    }

    #[test]
    fn test_no_scheme_but_has_port() {
        let result = RemoteEndpoint::try_parse("localhost:8888").unwrap();

        assert!(result.get_scheme().is_none());
        assert_eq!(result.get_host(), "localhost");
        assert_eq!(result.get_port_str(), Some("8888"));
    }

    #[test]
    fn test_no_scheme_and_no_port() {
        let result = RemoteEndpoint::try_parse("localhost").unwrap();

        assert!(result.get_scheme().is_none());
        assert_eq!(result.get_host(), "localhost");
        assert_eq!(result.get_port_str(), None);
    }

    #[test]
    fn test_get_host_port_with_default_port() {
        let result = RemoteEndpoint::try_parse("localhost").unwrap();

        let host_port = result.get_host_port(Some(80));
        assert_eq!(host_port.as_str(), "localhost:80");
    }

    #[test]
    fn test_http_endpoint_with_path_and_query() {
        let result = RemoteEndpoint::try_parse("http://localhost:4343/test").unwrap();

        assert!(result.get_scheme().unwrap().is_http());
        assert_eq!(result.get_host(), "localhost");
        assert_eq!(result.get_port_str(), Some("4343"));
        assert_eq!(result.get_http_path_and_query(), Some("/test"));
    }

    #[test]
    fn test_ws_endpoint_with_path_and_query() {
        let result = RemoteEndpoint::try_parse("ws://localhost:4343/test").unwrap();

        assert!(result.get_scheme().unwrap().is_ws());
        assert_eq!(result.get_host(), "localhost");
        assert_eq!(result.get_port_str(), Some("4343"));
        assert_eq!(result.get_http_path_and_query(), Some("/test"));
    }

    #[test]
    fn test_wss_endpoint_with_path_and_query() {
        let result = RemoteEndpoint::try_parse("wss://localhost:4343/test").unwrap();

        assert!(result.get_scheme().unwrap().is_wss());
        assert_eq!(result.get_host(), "localhost");
        assert_eq!(result.get_port_str(), Some("4343"));
        assert_eq!(result.get_http_path_and_query(), Some("/test"));
    }

    #[test]
    fn test_wss_from_real_life() {
        let result =
            RemoteEndpoint::try_parse("wss://api-dev.tradelocker.com/brand-api/socket.io").unwrap();

        assert!(result.get_scheme().unwrap().is_wss());
        assert_eq!(
            result.get_host_port(Some(443)).as_str(),
            "api-dev.tradelocker.com:443"
        );

        assert_eq!(result.get_host(), "api-dev.tradelocker.com");
    }
}
