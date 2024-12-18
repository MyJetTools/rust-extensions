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

    pub fn get_default_port(&self) -> Option<u16> {
        match self {
            Scheme::Http => Some(80),
            Scheme::Https => Some(443),
            Scheme::Ws => Some(80),
            Scheme::Wss => Some(443),
            Scheme::UnixSocket => None,
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
    port_position: Option<usize>,
    http_path_and_query_position: usize,
    default_port: Option<u16>,
}

impl RemoteEndpointInner {
    pub fn try_parse(src: &str) -> Result<Self, String> {
        let mut scheme_name_end_position = None;

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
                        continue;
                    }

                    if c == '/' {
                        scheme_name_end_position = Some(pos - 1);
                        reading_mode = ReadingEndpointMode::ReadingSchemeLastSymbols;
                        continue;
                    }

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
                        port_position = Some(pos);
                    }

                    '/' => {
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

        let http_path_and_query_position = match http_path_and_query_position {
            Some(pos) => pos,
            None => src.len(),
        };

        if scheme_name_end_position.is_none() {
            return Ok(Self {
                scheme: None,
                host_position: 0,
                port_position,
                http_path_and_query_position,
                default_port: None,
            });
        }

        let scheme_name_end_position = scheme_name_end_position.unwrap();

        let scheme = &src[..scheme_name_end_position];

        if let Some(scheme) = Scheme::try_parse(scheme) {
            return Ok(Self {
                scheme: Some(scheme),
                host_position: scheme_name_end_position + scheme.host_postfix_len(),
                port_position,
                http_path_and_query_position,
                default_port: None,
            });
        }

        panic!("Invalid scheme name {}", scheme);
    }

    pub fn get_host<'s>(&self, src: &'s str) -> &'s str {
        if let Some(port_position) = self.port_position {
            return &src[self.host_position..port_position];
        }

        return &src[self.host_position..self.http_path_and_query_position];
    }

    pub fn get_port_str<'s>(&self, src: &'s str) -> Option<&'s str> {
        if let Some(port_position) = self.port_position {
            Some(&src[port_position + 1..self.http_path_and_query_position])
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

    pub fn get_host_port(&self, src: &str) -> ShortString {
        let mut result = ShortString::new_empty();
        result.push_str(&src[self.host_position..self.http_path_and_query_position]);
        if self.port_position.is_some() {
            return result;
        }

        if let Some(default_port) = self.default_port {
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

    pub fn set_default_port(&mut self, default_port: u16) {
        self.inner.default_port = Some(default_port);
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

    pub fn get_host_port(&self) -> ShortString {
        self.inner.get_host_port(self.host_str)
    }

    pub fn get_http_path_and_query(&self) -> Option<&str> {
        if self.inner.http_path_and_query_position == self.host_str.len() {
            return None;
        }
        Some(&self.host_str[self.inner.http_path_and_query_position..])
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

    pub fn set_default_port(&mut self, default_port: u16) {
        self.inner.default_port = default_port.into();
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

    pub fn get_host_port(&self) -> ShortString {
        self.inner.get_host_port(&self.host_str)
    }

    pub fn as_str(&self) -> &str {
        &self.host_str
    }

    pub fn get_http_path_and_query(&self) -> Option<&str> {
        if self.inner.http_path_and_query_position == self.host_str.len() {
            return None;
        }
        Some(&self.host_str[self.inner.http_path_and_query_position..])
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
        let mut result = RemoteEndpoint::try_parse("localhost").unwrap();
        result.set_default_port(80);

        let host_port = result.get_host_port();
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
        let mut result =
            RemoteEndpoint::try_parse("wss://api-dev.tradelocker.com/brand-api/socket.io").unwrap();

        result.set_default_port(443);

        assert!(result.get_scheme().unwrap().is_wss());
        assert_eq!(
            result.get_host_port().as_str(),
            "api-dev.tradelocker.com:443"
        );

        assert_eq!(result.get_host(), "api-dev.tradelocker.com");
    }

    #[test]
    fn test_with_ip() {
        let mut result =
            RemoteEndpoint::try_parse("http://127.0.0.1:9191/first/next/other").unwrap();

        result.set_default_port(80);

        assert!(result.get_scheme().unwrap().is_http());
        assert_eq!(result.get_host_port().as_str(), "127.0.0.1:9191");

        assert_eq!(result.get_host(), "127.0.0.1");

        assert_eq!(
            result.get_http_path_and_query().unwrap(),
            "/first/next/other"
        );
    }

    #[test]
    fn test_with_ip_without_scheme_and_path_and_query() {
        let mut result = RemoteEndpoint::try_parse("127.0.0.1:9191").unwrap();
        result.set_default_port(80);

        assert!(result.get_scheme().is_none());
        assert_eq!(result.get_host_port().as_str(), "127.0.0.1:9191");

        assert_eq!(result.get_host(), "127.0.0.1");

        assert!(result.get_http_path_and_query().is_none());
    }
}
