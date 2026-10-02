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

/// Reads a host with its port from `from` on. Returns the position of the port
/// separator — the last ':' before the host ends — and the position the host, with
/// its port, ends at: the first '/', '?' or '#', or the end of the address. In the
/// path of a socket file a '?' and a '#' are characters of the path.
///
/// An IPv6 literal is written in brackets, and the colons inside them belong to the
/// address: only a colon after the closing bracket separates the port.
fn read_host_and_port(
    bytes: &[u8],
    from: usize,
    mut inside_brackets: bool,
    is_socket_path: bool,
) -> (Option<usize>, usize) {
    let mut port_position = None;

    for (pos, b) in bytes.iter().enumerate().skip(from) {
        match b {
            b']' => inside_brackets = false,
            b':' if !inside_brackets => port_position = Some(pos),
            b'/' => return (port_position, pos),
            b'?' | b'#' if !is_socket_path => return (port_position, pos),
            _ => {}
        }
    }

    (port_position, bytes.len())
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
    fn new(
        scheme: Option<Scheme>,
        host_position: usize,
        port_position: Option<usize>,
        http_path_and_query_position: usize,
    ) -> Self {
        Self {
            scheme,
            host_position,
            port_position,
            http_path_and_query_position,
            default_port: None,
        }
    }

    /// Splits an address into the scheme, the host, the port and the http path with
    /// the query.
    ///
    /// * The scheme is looked for at the start of the address only: a `:/` further
    ///   on belongs to the path or to the query.
    /// * The host ends at the first '/', '?' or '#', with a scheme and without one.
    ///   Everything from there on — a fragment too — is the path with the query.
    /// * An IPv6 literal in brackets is a host as a whole, with its brackets.
    /// * An address that starts with '/' or '~' is the path of a socket file, and
    ///   all of it is the host.
    pub fn try_parse(src: &str) -> Result<Self, String> {
        // The delimiters are ASCII, and no ASCII byte ever appears inside a
        // multi-byte UTF-8 sequence, so every position recorded here and sliced
        // with later is a char boundary even when the host or the path contains
        // multi-byte characters.
        let bytes = src.as_bytes();

        // The first ':' of an IPv6 literal is one of its own, so an address that
        // starts with a literal has no scheme to look for.
        if bytes.first() == Some(&b'[') {
            let (port_position, host_end) = read_host_and_port(bytes, 0, true, false);
            return Ok(Self::new(None, 0, port_position, host_end));
        }

        // `/var/run/docker.sock`, `~/docker.sock` — the path of a socket file. Its
        // slashes are its own, so a '/', a '?' or a '#' does not end it.
        let is_socket_path = matches!(bytes.first(), Some(b'/' | b'~'));

        let mut first_colon = None;

        for (pos, b) in bytes.iter().enumerate() {
            match b {
                b':' => {
                    first_colon = Some(pos);
                    break;
                }
                // The host ended before any ':' — there is neither a scheme nor a
                // port, and whatever ':' comes later belongs to the path or the query.
                b'/' | b'?' | b'#' if !is_socket_path => {
                    return Ok(Self::new(None, 0, None, pos));
                }
                _ => {}
            }
        }

        let Some(first_colon) = first_colon else {
            return Ok(Self::new(None, 0, None, src.len()));
        };

        match bytes.get(first_colon + 1) {
            Some(b'/') => {
                let scheme_name = &src[..first_colon];

                let Some(scheme) = Scheme::try_parse(scheme_name) else {
                    return Err(format!("Invalid scheme name {}", scheme_name));
                };

                if scheme.is_unix_socket() {
                    // A unix-socket URL carries an absolute socket path where the host
                    // would normally be. Keep exactly one leading '/' regardless of how
                    // many slashes follow the scheme separator, so `unix://path`,
                    // `unix:///path`, `unix+http://path` and `http+unix://path` all
                    // resolve to the very same socket path.
                    let host_position = unix_socket_host_position(src, first_colon);
                    return Ok(Self::new(Some(scheme), host_position, None, src.len()));
                }

                // Any other scheme ends with `://`. With one slash — `http:/host` —
                // the address has no scheme: `http` is its host, and the path starts
                // at the slash.
                if bytes.get(first_colon + 2) != Some(&b'/') {
                    return Ok(Self::new(None, 0, Some(first_colon), first_colon + 1));
                }

                // The host starts right after `://`, and its first character is read
                // like any other: `http://:8080` has an empty host and a port,
                // `http:///path` an empty host and a path.
                let host_position = first_colon + 3;
                let inside_brackets = bytes.get(host_position) == Some(&b'[');
                let (port_position, host_end) =
                    read_host_and_port(bytes, host_position, inside_brackets, false);

                Ok(Self::new(
                    Some(scheme),
                    host_position,
                    port_position,
                    host_end,
                ))
            }
            // `host:8080` — the port is the digits, and whatever follows them is the
            // path and the query.
            Some(b) if b.is_ascii_digit() => {
                let port_len = bytes[first_colon + 1..]
                    .iter()
                    .take_while(|b| b.is_ascii_digit())
                    .count();

                Ok(Self::new(
                    None,
                    0,
                    Some(first_colon),
                    first_colon + 1 + port_len,
                ))
            }
            Some(b'?' | b'#') if !is_socket_path => Ok(Self::new(None, 0, None, first_colon + 1)),
            // Neither a scheme nor a port follows the first ':', so it stays in the
            // host together with the character after it, and from there on the last
            // ':' is the port separator.
            Some(_) => {
                let (port_position, host_end) =
                    read_host_and_port(bytes, first_colon + 2, false, is_socket_path);

                Ok(Self::new(None, 0, port_position, host_end))
            }
            None => Ok(Self::new(None, 0, None, src.len())),
        }
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

    /// `host:port` to connect to. An address that has no port gets the default one
    /// of its scheme, or, with no scheme, the one set by `set_default_port`.
    ///
    /// The result is a `ShortString`, so this panics when it is longer than 255
    /// bytes. A caller that takes the address from outside has to check its length
    /// first.
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

    /// Panics when `host:port` is longer than 255 bytes, see
    /// [`RemoteEndpointInner::get_host_port`].
    pub fn get_host_port(&self) -> ShortString {
        self.inner.get_host_port(self.host_str)
    }

    /// What follows the host and the port, exactly as it is written in the address.
    /// With no path there it starts with the '?' of the query or the '#' of the
    /// fragment rather than with a '/', so it is not a request target as it is.
    ///
    /// `None` when nothing follows the host, and for a unix socket.
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

    /// Panics when `host:port` is longer than 255 bytes, see
    /// [`RemoteEndpointInner::get_host_port`].
    pub fn get_host_port(&self) -> ShortString {
        self.inner.get_host_port(&self.host_str)
    }

    pub fn as_str(&self) -> &str {
        &self.host_str
    }

    /// See [`RemoteEndpoint::get_http_path_and_query`].
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
