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
        } else if src.eq_ignore_ascii_case("http+unix")
            || src.eq_ignore_ascii_case("unix+http")
            || src.eq_ignore_ascii_case("unix")
        {
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

/// Returns the byte offset of the last '/' in the run of slashes that immediately
/// follows the scheme separator (`:`). Slicing the source from here yields the
/// unix socket path with exactly one leading slash, no matter whether the URL was
/// written with one, two or three slashes after the scheme.
///
/// `colon_position` is the byte offset of the ':' that terminates the scheme name
/// (always an ASCII byte). A unix-socket scheme is only detected when at least one
/// '/' follows the ':', so the returned offset always points at a '/'.
fn unix_socket_host_position(src: &str, colon_position: usize) -> usize {
    let bytes = src.as_bytes();
    let mut position = colon_position + 1;
    let mut last_slash = position;
    while position < bytes.len() && bytes[position] == b'/' {
        last_slash = position;
        position += 1;
    }
    last_slash
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

        // `char_indices` yields the byte offset of each char, so every position we
        // record and later slice with is a valid UTF-8 boundary even when the host
        // or path contains multi-byte characters. All the reference chars we do
        // arithmetic against (`:`, `/`) are ASCII, so `pos - 1` stays byte-safe.
        for (pos, c) in src.char_indices() {
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
            let host_position = if scheme.is_unix_socket() {
                // A unix-socket URL carries an absolute socket path where the host
                // would normally be. Keep exactly one leading '/' regardless of how
                // many slashes follow the scheme separator, so `unix://path`,
                // `unix:///path`, `unix+http://path` and `http+unix://path` all
                // resolve to the very same socket path.
                unix_socket_host_position(src, scheme_name_end_position)
            } else {
                scheme_name_end_position + scheme.host_postfix_len()
            };

            return Ok(Self {
                scheme: Some(scheme),
                host_position,
                port_position,
                http_path_and_query_position,
                default_port: None,
            });
        }

        return Err(format!("Invalid scheme name {}", scheme));
    }

    fn is_unix_socket(&self) -> bool {
        if let Some(scheme) = self.scheme {
            return scheme.is_unix_socket();
        }

        false
    }

    pub fn get_host<'s>(&self, src: &'s str) -> &'s str {
        if self.is_unix_socket() {
            return &src[self.host_position..];
        }

        if let Some(port_position) = self.port_position {
            return &src[self.host_position..port_position];
        }

        return &src[self.host_position..self.http_path_and_query_position];
    }

    pub fn get_port_str<'s>(&self, src: &'s str) -> Option<&'s str> {
        if self.is_unix_socket() {
            return None;
        }

        if let Some(port_position) = self.port_position {
            Some(&src[port_position + 1..self.http_path_and_query_position])
        } else {
            None
        }
    }

    pub fn get_port(&self, src: &str) -> Option<u16> {
        if self.is_unix_socket() {
            return None;
        }

        if let Some(port_str) = self.get_port_str(src) {
            // The port comes straight from user URL input. A malformed one yields
            // `None` (not a panic, and without silently substituting a default) so
            // callers can decide how to handle it.
            return port_str.parse().ok();
        }

        // No explicit port: fall back to the default the same way `get_host_port`
        // does — a known scheme supplies its own default (80/443/…), otherwise the
        // port configured via `set_default_port` is used.
        if let Some(scheme) = self.scheme {
            return scheme.get_default_port();
        }

        self.default_port
    }

    pub fn get_host_port(&self, src: &str) -> ShortString {
        let mut result = ShortString::new_empty();

        if self.is_unix_socket() {
            let host_as_str = &src[self.host_position..];
            result.push_str(host_as_str);
            return result;
        }

        result.push_str(&src[self.host_position..self.http_path_and_query_position]);
        if self.port_position.is_some() {
            return result;
        }

        if let Some(scheme) = self.scheme {
            if let Some(default_port) = scheme.get_default_port() {
                result.push_str(":");
                result.push_str(default_port.to_string().as_str());
            }

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
        if self.inner.is_unix_socket() {
            return None;
        }

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

    pub fn to_ref<'s>(&'s self) -> RemoteEndpoint<'s> {
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
        if self.inner.is_unix_socket() {
            return None;
        }

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

    #[test]
    fn test_unix_socket_case() {
        let result = RemoteEndpoint::try_parse("http+unix://var/run/docker.sock").unwrap();

        assert!(result.get_scheme().unwrap().is_unix_socket());
        assert_eq!(result.get_host(), "/var/run/docker.sock");

        assert_eq!(result.get_host_port().as_str(), "/var/run/docker.sock");

        assert!(result.get_http_path_and_query().is_none());

        let owned = result.to_owned();

        assert!(owned.get_scheme().unwrap().is_unix_socket());
        assert_eq!(owned.get_host(), "/var/run/docker.sock");

        assert_eq!(owned.get_host_port().as_str(), "/var/run/docker.sock");

        assert!(owned.get_http_path_and_query().is_none());
    }

    #[test]
    fn test_unix_socket_scheme_aliases_are_equivalent() {
        // Every documented spelling of a unix-socket URL must resolve to the same
        // endpoint, including the natural `unix:///...` form with three slashes.
        for form in [
            "unix:///var/run/docker.sock",
            "unix://var/run/docker.sock",
            "unix+http://var/run/docker.sock",
            "http+unix://var/run/docker.sock",
        ] {
            let ep = RemoteEndpoint::try_parse(form)
                .unwrap_or_else(|e| panic!("{form} should parse, got {e}"));

            assert!(
                ep.get_scheme().unwrap().is_unix_socket(),
                "{form} must be a unix socket"
            );
            assert_eq!(ep.get_host(), "/var/run/docker.sock", "host for {form}");
            assert_eq!(
                ep.get_host_port().as_str(),
                "/var/run/docker.sock",
                "host_port for {form}"
            );
            assert_eq!(ep.get_port(), None, "port for {form}");
            assert!(
                ep.get_http_path_and_query().is_none(),
                "path for {form}"
            );

            // The owned copy must round-trip to the same values.
            let owned = ep.to_owned();
            assert_eq!(owned.get_host(), "/var/run/docker.sock", "owned host for {form}");
            assert!(owned.get_scheme().unwrap().is_unix_socket());
        }
    }

    #[test]
    fn test_get_port_falls_back_to_default() {
        // A known scheme supplies its own default port even without set_default_port.
        let ep = RemoteEndpoint::try_parse("http://host").unwrap();
        assert_eq!(ep.get_port(), Some(80));

        // ...and supplying HTTP_DEFAULT_PORT via set_default_port keeps it Some(80).
        let mut ep = RemoteEndpoint::try_parse("http://host").unwrap();
        ep.set_default_port(80);
        assert_eq!(ep.get_port(), Some(80));

        // The scheme default wins over a mismatched configured default.
        let mut ep = RemoteEndpoint::try_parse("https://host").unwrap();
        ep.set_default_port(80);
        assert_eq!(ep.get_port(), Some(443));

        // No scheme: the configured default is used.
        let mut ep = RemoteEndpoint::try_parse("host").unwrap();
        ep.set_default_port(80);
        assert_eq!(ep.get_port(), Some(80));

        // No scheme and no default configured: still None.
        let ep = RemoteEndpoint::try_parse("host").unwrap();
        assert_eq!(ep.get_port(), None);

        // An explicit port always wins over any default.
        let mut ep = RemoteEndpoint::try_parse("http://host:9000").unwrap();
        ep.set_default_port(80);
        assert_eq!(ep.get_port(), Some(9000));
    }

    #[test]
    fn test_get_port_with_malformed_port_does_not_panic() {
        // A non-numeric port comes straight from user URL input and must yield
        // `None` instead of aborting the process.
        let ep = RemoteEndpoint::try_parse("http://host:abc").unwrap();
        assert_eq!(ep.get_port(), None);

        // Same with a trailing path after the malformed port.
        let ep = RemoteEndpoint::try_parse("http://host:abc/path").unwrap();
        assert_eq!(ep.get_port(), None);

        // Out-of-range (overflows u16) is also just `None`, not a panic.
        let ep = RemoteEndpoint::try_parse("http://host:99999").unwrap();
        assert_eq!(ep.get_port(), None);

        // A valid explicit port still parses.
        let ep = RemoteEndpoint::try_parse("http://host:8080").unwrap();
        assert_eq!(ep.get_port(), Some(8080));

        // A portless endpoint still returns the scheme default.
        let ep = RemoteEndpoint::try_parse("http://host").unwrap();
        assert_eq!(ep.get_port(), Some(80));

        // Owned variant behaves identically.
        let owned = RemoteEndpoint::try_parse("http://host:abc").unwrap().to_owned();
        assert_eq!(owned.get_port(), None);
    }

    #[test]
    fn test_non_ascii_host_and_path() {
        // Multi-byte characters in the host and the path must not panic or corrupt
        // the parsed slices (positions are byte offsets, so slicing stays on
        // UTF-8 boundaries).
        let ep = RemoteEndpoint::try_parse("http://münchen.example/pàth/reçu").unwrap();
        assert!(ep.get_scheme().unwrap().is_http());
        assert_eq!(ep.get_host(), "münchen.example");
        assert_eq!(ep.get_http_path_and_query(), Some("/pàth/reçu"));
        assert_eq!(ep.get_host_port().as_str(), "münchen.example:80");

        // Multi-byte host with an explicit port and a multi-byte path.
        let ep = RemoteEndpoint::try_parse("https://münchen.example:8443/pàth").unwrap();
        assert_eq!(ep.get_host(), "münchen.example");
        assert_eq!(ep.get_port(), Some(8443));
        assert_eq!(ep.get_port_str(), Some("8443"));
        assert_eq!(ep.get_http_path_and_query(), Some("/pàth"));

        // Multi-byte host, no scheme, explicit port (this exact case used to panic
        // because the ':' byte offset differed from its char index).
        let ep = RemoteEndpoint::try_parse("münchen.example:8080").unwrap();
        assert!(ep.get_scheme().is_none());
        assert_eq!(ep.get_host(), "münchen.example");
        assert_eq!(ep.get_port(), Some(8080));

        // Non-ascii unix socket path.
        let ep = RemoteEndpoint::try_parse("unix:///var/run/société.sock").unwrap();
        assert!(ep.get_scheme().unwrap().is_unix_socket());
        assert_eq!(ep.get_host(), "/var/run/société.sock");
    }

    #[test]
    fn test_plain_ascii_http_https_regression() {
        // No behaviour change for plain ASCII http/https endpoints.
        let ep = RemoteEndpoint::try_parse("http://localhost:8000/path?q=1").unwrap();
        assert!(ep.get_scheme().unwrap().is_http());
        assert_eq!(ep.get_host(), "localhost");
        assert_eq!(ep.get_port(), Some(8000));
        assert_eq!(ep.get_port_str(), Some("8000"));
        assert_eq!(ep.get_http_path_and_query(), Some("/path?q=1"));
        assert_eq!(ep.get_host_port().as_str(), "localhost:8000");

        let ep = RemoteEndpoint::try_parse("https://example.com").unwrap();
        assert!(ep.get_scheme().unwrap().is_https());
        assert_eq!(ep.get_host(), "example.com");
        assert_eq!(ep.get_port_str(), None);
        assert_eq!(ep.get_host_port().as_str(), "example.com:443");
    }
}
