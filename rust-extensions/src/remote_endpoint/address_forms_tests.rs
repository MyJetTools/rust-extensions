//! Forms of an address, and what `RemoteEndpoint` makes of each.
//!
//! A row pins every getter at once, so a change in how a form is split shows up as a
//! changed row.

use super::{RemoteEndpoint, RemoteEndpointHostString, Scheme, SshRemoteEndpoint};

fn caught(f: impl FnOnce() -> String) -> String {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
        .unwrap_or_else(|_| "<panic>".to_string())
}

fn scheme_name(scheme: Option<Scheme>) -> &'static str {
    match scheme {
        None => "-",
        Some(Scheme::Http) => "http",
        Some(Scheme::Https) => "https",
        Some(Scheme::Ws) => "ws",
        Some(Scheme::Wss) => "wss",
        Some(Scheme::UnixSocket) => "unix",
    }
}

/// Every getter of the endpoint in one line; one that panics shows as `<panic>`, and an
/// address `try_parse` refuses shows the error.
///
/// `port` is what the address has written in the port's place, `port_no` is what
/// `get_port()` makes of it. No default port is set, so where the address has no port
/// `port_no` and `host_port` show the default one of the scheme, if it has one.
fn snapshot(src: &str) -> String {
    let endpoint = match RemoteEndpoint::try_parse(src) {
        Ok(endpoint) => endpoint,
        Err(err) => return format!("Err({err})"),
    };

    let result = format!(
        "{} host={} port={} port_no={} host_port={} path={}",
        scheme_name(endpoint.get_scheme()),
        caught(|| endpoint.get_host().to_string()),
        caught(|| endpoint.get_port_str().unwrap_or("-").to_string()),
        caught(|| match endpoint.get_port() {
            Some(port) => port.to_string(),
            None => "-".to_string(),
        }),
        caught(|| endpoint.get_host_port().to_string()),
        caught(|| endpoint
            .get_http_path_and_query()
            .unwrap_or("-")
            .to_string()),
    );

    // The owned endpoint reads the address through the same positions.
    let owned = endpoint.to_owned();
    let owned = format!(
        "{} host={} port={} port_no={} host_port={} path={}",
        scheme_name(owned.get_scheme()),
        caught(|| owned.get_host().to_string()),
        caught(|| owned.get_port_str().unwrap_or("-").to_string()),
        caught(|| match owned.get_port() {
            Some(port) => port.to_string(),
            None => "-".to_string(),
        }),
        caught(|| owned.get_host_port().to_string()),
        caught(|| owned.get_http_path_and_query().unwrap_or("-").to_string()),
    );
    assert_eq!(
        result, owned,
        "{src:?}: the owned endpoint reads differently"
    );

    result
}

fn assert_forms(forms: &[(&str, &str)]) {
    for (src, expected) in forms {
        assert_eq!(snapshot(src), *expected, "{src:?}");
    }
}

/// For comparison with the forms below.
#[test]
fn ordinary_addresses() {
    assert_forms(&[
        (
            "localhost:8080/api?a=b",
            "- host=localhost port=8080 port_no=8080 host_port=localhost:8080 path=/api?a=b",
        ),
        (
            "HTTP://Host:8080",
            "http host=Host port=8080 port_no=8080 host_port=Host:8080 path=-",
        ),
        (
            "https://host/path?a=b",
            "https host=host port=- port_no=443 host_port=host:443 path=/path?a=b",
        ),
        (
            "unix:8080",
            "- host=unix port=8080 port_no=8080 host_port=unix:8080 path=-",
        ),
    ]);
}

/// A scheme `Scheme` does not know is refused, with two slashes after it and with one.
#[test]
fn an_unknown_scheme() {
    assert_forms(&[
        ("ftp://host", "Err(Invalid scheme name ftp)"),
        ("ftp:/host", "Err(Invalid scheme name ftp)"),
    ]);
}

/// A scheme is looked for at the start of the address only. A `://` further on belongs
/// to the path or the query, and an address that has one there and no scheme of its own
/// is read like any other address with no scheme.
#[test]
fn a_scheme_is_only_what_the_address_starts_with() {
    assert_forms(&[
        (
            "host/path?next=http://other",
            "- host=host port=- port_no=- host_port=host path=/path?next=http://other",
        ),
        (
            "host?next=http://other",
            "- host=host port=- port_no=- host_port=host path=?next=http://other",
        ),
        (
            "host:8080/path?next=http://other",
            "- host=host port=8080 port_no=8080 host_port=host:8080 path=/path?next=http://other",
        ),
        (
            "http://host/path?next=http://other",
            "http host=host port=- port_no=80 host_port=host:80 path=/path?next=http://other",
        ),
    ]);
}

/// The host ends at the first `/` and at the first `?`, with a scheme and without one —
/// so a `:` in the path or in the query is not a port separator.
///
/// With a query and no path `get_http_path_and_query()` starts with the `?`: it is a
/// slice of the address, and the address has no `/` there. A caller that sends it as a
/// request target has to put the `/` in front itself.
#[test]
fn the_host_ends_where_the_path_or_the_query_starts() {
    assert_forms(&[
        (
            "localhost/api",
            "- host=localhost port=- port_no=- host_port=localhost path=/api",
        ),
        (
            "localhost?a=b",
            "- host=localhost port=- port_no=- host_port=localhost path=?a=b",
        ),
        (
            "localhost/api?time=10:30",
            "- host=localhost port=- port_no=- host_port=localhost path=/api?time=10:30",
        ),
        (
            "localhost:8080?a=b",
            "- host=localhost port=8080 port_no=8080 host_port=localhost:8080 path=?a=b",
        ),
        (
            "http://host?a=b",
            "http host=host port=- port_no=80 host_port=host:80 path=?a=b",
        ),
        (
            "http://host?t=10:30",
            "http host=host port=- port_no=80 host_port=host:80 path=?t=10:30",
        ),
        (
            "http://host:8080?a=b",
            "http host=host port=8080 port_no=8080 host_port=host:8080 path=?a=b",
        ),
        (
            "http://host/api?time=10:30",
            "http host=host port=- port_no=80 host_port=host:80 path=/api?time=10:30",
        ),
    ]);
}

/// The first character after the scheme is a delimiter like any other, so an address
/// that names no host has an empty one, with the port, the path, the query and the
/// fragment each in its own place.
#[test]
fn an_address_with_no_host() {
    assert_forms(&[
        ("", "- host= port=- port_no=- host_port= path=-"),
        (
            "http://",
            "http host= port=- port_no=80 host_port=:80 path=-",
        ),
        (
            "http://:8080",
            "http host= port=8080 port_no=8080 host_port=:8080 path=-",
        ),
        (
            "http:///path",
            "http host= port=- port_no=80 host_port=:80 path=/path",
        ),
        (
            "http://?a=b",
            "http host= port=- port_no=80 host_port=:80 path=?a=b",
        ),
        (
            "http://#frag",
            "http host= port=- port_no=80 host_port=:80 path=#frag",
        ),
    ]);
}

/// The colons inside the brackets belong to the address, and only one after the closing
/// bracket separates the port. The brackets stay in the host.
#[test]
fn an_ipv6_literal_is_a_host() {
    assert_forms(&[
        (
            "http://[::1]",
            "http host=[::1] port=- port_no=80 host_port=[::1]:80 path=-",
        ),
        (
            "http://[::1]/path",
            "http host=[::1] port=- port_no=80 host_port=[::1]:80 path=/path",
        ),
        (
            "http://[2001:db8::1]/path?a=b",
            "http host=[2001:db8::1] port=- port_no=80 host_port=[2001:db8::1]:80 path=/path?a=b",
        ),
        (
            "http://[::1]:8080",
            "http host=[::1] port=8080 port_no=8080 host_port=[::1]:8080 path=-",
        ),
        (
            "http://[::1]:8080/path?a=b",
            "http host=[::1] port=8080 port_no=8080 host_port=[::1]:8080 path=/path?a=b",
        ),
        (
            "[::1]",
            "- host=[::1] port=- port_no=- host_port=[::1] path=-",
        ),
        (
            "[2001:db8::1]",
            "- host=[2001:db8::1] port=- port_no=- host_port=[2001:db8::1] path=-",
        ),
        (
            "[2001:0db8::1]:8080/path",
            "- host=[2001:0db8::1] port=8080 port_no=8080 host_port=[2001:0db8::1]:8080 path=/path",
        ),
        // A bracket that is never closed: there is no port to tell from the address.
        (
            "http://[::1",
            "http host=[::1 port=- port_no=80 host_port=[::1:80 path=-",
        ),
    ]);
}

/// A scheme other than a unix one ends with `://`. With one slash after it the address
/// has no scheme: the name is its host, and the path starts at the slash — the way
/// `UrlBuilder` of my-http-utils reads it.
#[test]
fn a_scheme_with_one_slash() {
    assert_forms(&[
        (
            "http:/",
            "- host=http port= port_no=- host_port=http: path=/",
        ),
        (
            "http:/é",
            "- host=http port= port_no=- host_port=http: path=/é",
        ),
        (
            "http:/host",
            "- host=http port= port_no=- host_port=http: path=/host",
        ),
        (
            "http:/host/x",
            "- host=http port= port_no=- host_port=http: path=/host/x",
        ),
    ]);
}

/// With no scheme the first `:` separates the port only when a digit follows it, and
/// then whatever follows the digits is the path. Otherwise that `:` stays in the host,
/// and so does a second one right after it.
///
/// Pinned as it is: this is how an address with no scheme has always been read.
#[test]
fn the_first_colon_of_an_address_with_no_scheme() {
    assert_forms(&[
        (
            "host:abc",
            "- host=host:abc port=- port_no=- host_port=host:abc path=-",
        ),
        (
            "http://host:abc",
            "http host=host port=abc port_no=- host_port=host:abc path=-",
        ),
        (
            "host:",
            "- host=host: port=- port_no=- host_port=host: path=-",
        ),
        (
            "host:8080abc",
            "- host=host port=8080 port_no=8080 host_port=host:8080 path=abc",
        ),
        ("::1", "- host=::1 port=- port_no=- host_port=::1 path=-"),
        (
            "2001:db8::1",
            "- host=2001:db8: port=1 port_no=1 host_port=2001:db8::1 path=-",
        ),
    ]);
}

/// A path with no scheme is the path of a socket file: all of it is the host.
#[test]
fn a_bare_unix_socket_path() {
    assert_forms(&[
        (
            "/var/run/docker.sock",
            "- host=/var/run/docker.sock port=- port_no=- host_port=/var/run/docker.sock path=-",
        ),
        (
            "~/docker.sock",
            "- host=~/docker.sock port=- port_no=- host_port=~/docker.sock path=-",
        ),
        // A '?' and a '#' are characters a file name may have.
        (
            "/var/run/docker.sock?a=b",
            "- host=/var/run/docker.sock?a=b port=- port_no=- host_port=/var/run/docker.sock?a=b path=-",
        ),
        (
            "/var/run/docker#1.sock",
            "- host=/var/run/docker#1.sock port=- port_no=- host_port=/var/run/docker#1.sock path=-",
        ),
    ]);
}

/// Every spelling of the unix scheme, in any case, with any number of slashes after
/// it, names the same socket — a path with one leading slash.
#[test]
fn every_spelling_of_the_unix_scheme() {
    for src in [
        "unix:///var/run/docker.sock",
        "unix://var/run/docker.sock",
        "unix:/var/run/docker.sock",
        "unix+http:///var/run/docker.sock",
        "unix+http://var/run/docker.sock",
        "http+unix://var/run/docker.sock",
        "HTTP+UNIX:/var/run/docker.sock",
    ] {
        assert_eq!(
            snapshot(src),
            "unix host=/var/run/docker.sock port=- port_no=- host_port=/var/run/docker.sock path=-",
            "{src:?}"
        );
    }
}

/// `UrlBuilder` of my-http-utils writes the http path after the socket path, with a
/// `:` between them. Here it is not split off: it stays in the host of an address with
/// a unix scheme, and makes a bare path read as a scheme.
///
/// Left as it was on purpose: nothing about a unix socket address is read differently.
#[test]
fn a_unix_socket_with_an_http_path() {
    assert_forms(&[
        (
            "unix:///var/run/docker.sock:/containers/json?all=true",
            "unix host=/var/run/docker.sock:/containers/json?all=true port=- port_no=- host_port=/var/run/docker.sock:/containers/json?all=true path=-",
        ),
        (
            "/var/run/docker.sock:/containers/json",
            "Err(Invalid scheme name /var/run/docker.sock)",
        ),
    ]);
}

/// A fragment is a part of the path: `#` ends the host and the port the way `/` and `?`
/// do, and everything from it on stays in `get_http_path_and_query()` as it is written.
/// Nothing is cut off.
#[test]
fn a_fragment_is_a_part_of_the_path() {
    assert_forms(&[
        (
            "http://host#frag",
            "http host=host port=- port_no=80 host_port=host:80 path=#frag",
        ),
        (
            "http://host:8080#frag",
            "http host=host port=8080 port_no=8080 host_port=host:8080 path=#frag",
        ),
        (
            "http://host/path#frag",
            "http host=host port=- port_no=80 host_port=host:80 path=/path#frag",
        ),
        (
            "http://host?a=b#frag",
            "http host=host port=- port_no=80 host_port=host:80 path=?a=b#frag",
        ),
        (
            "http://[::1]#frag",
            "http host=[::1] port=- port_no=80 host_port=[::1]:80 path=#frag",
        ),
        (
            "host#frag",
            "- host=host port=- port_no=- host_port=host path=#frag",
        ),
        (
            "host:8080#frag",
            "- host=host port=8080 port_no=8080 host_port=host:8080 path=#frag",
        ),
        // A ':' after the '#' is in the fragment, not a port separator.
        (
            "http://host:80#a:1",
            "http host=host port=80 port_no=80 host_port=host:80 path=#a:1",
        ),
    ]);
}

/// `get_host_port()` returns a `ShortString`, which holds 255 bytes and panics past
/// them: `host:80` fits with a host of 252 bytes and does not with one of 253.
///
/// Left as it is on purpose: the panic can not go without a change of the return type,
/// so it is described on the getter, and a caller that takes the address from outside
/// checks the length first.
#[test]
fn a_host_port_longer_than_a_short_string() {
    let fits = format!("http://{}", "h".repeat(252));
    let endpoint = RemoteEndpoint::try_parse(&fits).unwrap();
    assert_eq!(endpoint.get_host().len(), 252);
    assert_eq!(endpoint.get_host_port().as_str().len(), 255);

    let too_long = format!("http://{}", "h".repeat(253));
    let endpoint = RemoteEndpoint::try_parse(&too_long).unwrap();
    assert_eq!(endpoint.get_host().len(), 253);
    assert_eq!(endpoint.get_port(), Some(80));
    assert_eq!(caught(|| endpoint.get_host_port().to_string()), "<panic>");
}

/// Pieces an address is put together from: every delimiter the parser looks at, the
/// schemes it knows and one it does not, and what goes between them.
const PIECES: [&str; 18] = [
    "http",
    "wss",
    "unix",
    "http+unix",
    "ftp",
    "host",
    "8080",
    "é",
    ":",
    "/",
    "://",
    "?",
    "#",
    "[",
    "]",
    "~",
    "@",
    "->",
];

/// Every address made of up to `len` of the pieces above.
fn addresses(len: usize) -> Vec<String> {
    let mut result = vec![String::new()];
    let mut from = 0;

    for _ in 0..len {
        let to = result.len();

        for index in from..to {
            for piece in PIECES {
                result.push(format!("{}{}", result[index], piece));
            }
        }

        from = to;
    }

    result
}

/// Calls every getter, and checks what has to hold for any address at all.
fn assert_is_sound(src: &str) {
    // Whatever these two make of the address, they must not panic on it.
    let _ = RemoteEndpointHostString::try_parse(src);
    let _ = SshRemoteEndpoint::try_parse(src);

    let Ok(endpoint) = RemoteEndpoint::try_parse(src) else {
        return;
    };

    let scheme = endpoint.get_scheme();
    let host = endpoint.get_host();
    let port = endpoint.get_port_str();
    let path = endpoint.get_http_path_and_query();
    let host_port = endpoint.get_host_port();
    let _ = endpoint.get_port();

    assert_eq!(endpoint.as_str(), src);

    let mut with_default_port = endpoint;
    with_default_port.set_default_port(80);
    let _ = with_default_port.get_port();
    let _ = with_default_port.get_host_port();

    let owned = endpoint.to_owned();
    assert_eq!(owned.get_host(), host, "{src:?}");
    assert_eq!(owned.get_port_str(), port, "{src:?}");
    assert_eq!(owned.get_http_path_and_query(), path, "{src:?}");
    assert_eq!(
        owned.get_host_port().as_str(),
        host_port.as_str(),
        "{src:?}"
    );

    if scheme.is_some_and(|scheme| scheme.is_unix_socket()) {
        // The path of the socket file is the rest of the address, with one slash
        // in front of it.
        assert!(host.starts_with('/'), "{src:?}");
        assert!(src.ends_with(host), "{src:?}");
        assert_eq!(host_port.as_str(), host, "{src:?}");
        assert_eq!(port, None, "{src:?}");
        assert_eq!(path, None, "{src:?}");
        return;
    }

    // The address is the scheme, the host, the port and the path with the query, in
    // that order and with nothing lost between them.
    let mut parsed = host.to_string();

    if let Some(port) = port {
        parsed.push(':');
        parsed.push_str(port);
    }

    assert!(host_port.as_str().starts_with(parsed.as_str()), "{src:?}");

    if let Some(path) = path {
        assert!(!path.is_empty(), "{src:?}");
        parsed.push_str(path);
    }

    match scheme {
        Some(scheme) => {
            // What is left in front of them is the scheme and its `://`.
            let scheme_in_address = src
                .strip_suffix(parsed.as_str())
                .and_then(|prefix| prefix.strip_suffix("://"))
                .and_then(Scheme::try_parse);

            assert_eq!(
                scheme_name(scheme_in_address),
                scheme_name(Some(scheme)),
                "{src:?}"
            );

            // After a scheme the path, the query or the fragment starts right where
            // the host ends.
            if let Some(path) = path {
                assert!(path.starts_with(['/', '?', '#']), "{src:?}");
            }
        }
        None => assert_eq!(parsed, src),
    }

    // The host ends where the path, the query or the fragment starts — unless the
    // address is the path of a socket file.
    if !src.starts_with(['/', '~']) {
        assert!(!host.contains(['/', '?', '#']), "{src:?}");
    }
}

/// No getter panics on any address `try_parse` accepts.
#[test]
fn no_address_makes_a_getter_panic() {
    for src in addresses(4) {
        assert_is_sound(&src);
    }
}
