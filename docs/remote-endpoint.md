# remote-endpoint

`rust_extensions::remote_endpoint` — parse a connection string into scheme, host, port and path, with no allocation for the borrowed forms.

## RemoteEndpoint

The scheme is optional. Without one there is no default port unless you set it. With a scheme, its default applies: http/ws 80, https/wss 443.

```rust
use rust_extensions::remote_endpoint::RemoteEndpoint;

let ep = RemoteEndpoint::try_parse("https://api.example.com/v1/orders?id=7").unwrap();
assert!(ep.get_scheme().unwrap().is_https());
assert_eq!(ep.get_host(), "api.example.com");
assert_eq!(ep.get_port_str(), None);
assert_eq!(ep.get_port(), Some(443)); // the scheme default
assert_eq!(ep.get_host_port().as_str(), "api.example.com:443");
assert_eq!(ep.get_http_path_and_query(), Some("/v1/orders?id=7"));
assert_eq!(ep.as_str(), "https://api.example.com/v1/orders?id=7");

let ep = RemoteEndpoint::try_parse("http://[::1]:8080/path").unwrap();
assert_eq!(ep.get_host(), "[::1]");
assert_eq!(ep.get_port(), Some(8080));

let mut ep = RemoteEndpoint::try_parse("10.0.0.5").unwrap();
assert!(ep.get_scheme().is_none());
assert_eq!(ep.get_port(), None);
ep.set_default_port(5432); // used only when there is neither a port nor a scheme
assert_eq!(ep.get_host_port().as_str(), "10.0.0.5:5432");

// A malformed port is None, not a panic
assert_eq!(RemoteEndpoint::try_parse("http://host:abc").unwrap().get_port(), None);
```

Unix sockets are spelled `http+unix:`, `unix+http:` or `unix:`, with any number of slashes. The host is then the socket path:

```rust
use rust_extensions::remote_endpoint::RemoteEndpoint;

let ep = RemoteEndpoint::try_parse("http+unix://var/run/docker.sock").unwrap();
assert!(ep.get_scheme().unwrap().is_unix_socket());
assert_eq!(ep.get_host(), "/var/run/docker.sock");
assert_eq!(ep.get_host_port().as_str(), "/var/run/docker.sock");
assert_eq!(ep.get_port(), None);
assert_eq!(ep.get_http_path_and_query(), None);
```

`get_host_port()` returns a `ShortString` and panics past 255 bytes, so check the length of an address that comes from outside.

`Scheme` is what `get_scheme()` returns; it also parses on its own, case-insensitively:

```rust
use rust_extensions::remote_endpoint::Scheme;

let https = Scheme::try_parse("HTTPS").unwrap();
assert!(https.is_https());
assert_eq!(https.get_default_port(), Some(443));

assert_eq!(Scheme::try_parse("ws").unwrap().get_default_port(), Some(80));
assert_eq!(Scheme::try_parse("unix").unwrap().get_default_port(), None); // http+unix, unix+http too
assert!(Scheme::try_parse("ftp").is_none());
```

`RemoteEndpointOwned` is the same thing holding its `String`:

```rust
use rust_extensions::remote_endpoint::{RemoteEndpoint, RemoteEndpointOwned};

let owned: RemoteEndpointOwned = RemoteEndpoint::try_parse("ws://localhost:9000").unwrap().to_owned();
let parsed = RemoteEndpointOwned::try_parse("ws://localhost:9000".to_string()).unwrap();

assert_eq!(owned.get_host_port().as_str(), parsed.get_host_port().as_str());
assert!(owned.to_ref().get_scheme().unwrap().is_ws());
```

## SshRemoteEndpoint

`user@host[:port]`, optionally prefixed with `ssh://` or `ssh:`. The port defaults to 22.

```rust
use rust_extensions::remote_endpoint::SshRemoteEndpoint;

let ssh = SshRemoteEndpoint::try_parse("ssh://deploy@bastion.example.com:2222").unwrap();
assert_eq!(ssh.get_user(), "deploy");
assert_eq!(ssh.get_host(), "bastion.example.com");
assert_eq!(ssh.get_port(), Some("2222"));
assert_eq!(ssh.get_host_port(), ("bastion.example.com", 2222));

assert_eq!(SshRemoteEndpoint::try_parse("deploy@bastion").unwrap().get_host_port(), ("bastion", 22));
assert!(SshRemoteEndpoint::try_parse("no-user-here").is_err());
let _owned = ssh.to_owned(); // SshRemoteEndpointOwned
```

## RemoteEndpointHostString — direct or through SSH

`ssh-part->endpoint` means "reach the endpoint through this SSH host". Without `->` the address is direct.

```rust
use rust_extensions::remote_endpoint::RemoteEndpointHostString;

match RemoteEndpointHostString::try_parse("ssh://deploy@bastion:22->http://10.0.0.5:8080").unwrap() {
    RemoteEndpointHostString::ViaSsh { ssh_remote_host, remote_host_behind_ssh } => {
        assert_eq!(ssh_remote_host.get_host(), "bastion");
        assert_eq!(remote_host_behind_ssh.get_host_port().as_str(), "10.0.0.5:8080");
    }
    RemoteEndpointHostString::Direct(_) => unreachable!(),
}

assert!(matches!(
    RemoteEndpointHostString::try_parse("https://example.com").unwrap(),
    RemoteEndpointHostString::Direct(_)
));
```
