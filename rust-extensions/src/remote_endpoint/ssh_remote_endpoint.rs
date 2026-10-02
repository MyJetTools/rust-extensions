use crate::str_utils::StrUtils;

const DEFAULT_SSH_PORT: u16 = 22;

// Positions are byte offsets in the string which was parsed
#[derive(Debug, Clone, Copy)]
pub struct SshRemoteEndpointInner {
    user_start: usize,
    user_separator: usize,
    port_separator: Option<usize>,
    port: u16,
}

impl SshRemoteEndpointInner {
    pub fn try_parse(src: &str) -> Result<Self, String> {
        let mut user_start = 0;

        if let Some(scheme_separator) = src.find(':') {
            let scheme = &src[..scheme_separator];

            if scheme.eq_case_insensitive("ssh") {
                user_start = scheme_separator + 1;
            }
        }

        if src[user_start..].starts_with("//") {
            user_start += 2;
        }

        let user_separator = match src[user_start..].rfind('@') {
            Some(pos) => user_start + pos,
            None => return Err(format!("Ssh string is wrong {src}")),
        };

        if src[user_start..user_separator].contains(':') {
            return Err(format!("Ssh string is wrong {src}"));
        }

        let host_start = user_separator + 1;

        match src[host_start..].find(':') {
            Some(pos) => {
                let port_separator = host_start + pos;
                let port = &src[port_separator + 1..];

                match port.parse::<u16>() {
                    Ok(port) => Ok(Self {
                        user_start,
                        user_separator,
                        port_separator: Some(port_separator),
                        port,
                    }),
                    Err(_) => Err(format!("Invalid port {port} of ssh string {src}")),
                }
            }
            None => Ok(Self {
                user_start,
                user_separator,
                port_separator: None,
                port: DEFAULT_SSH_PORT,
            }),
        }
    }

    pub fn get_user<'s>(&self, src: &'s str) -> &'s str {
        &src[self.user_start..self.user_separator]
    }

    pub fn get_host<'s>(&self, src: &'s str) -> &'s str {
        if let Some(port_separator) = self.port_separator {
            &src[self.user_separator + 1..port_separator]
        } else {
            &src[self.user_separator + 1..]
        }
    }

    pub fn get_port<'s>(&self, src: &'s str) -> Option<&'s str> {
        let port_separator = self.port_separator?;
        Some(&src[port_separator + 1..])
    }

    pub fn get_host_port<'s>(&self, src: &'s str) -> (&'s str, u16) {
        (self.get_host(src), self.port)
    }
}

pub struct SshRemoteEndpoint<'s> {
    src: &'s str,
    inner: SshRemoteEndpointInner,
}

impl<'s> SshRemoteEndpoint<'s> {
    pub fn try_parse(src: &'s str) -> Result<Self, String> {
        let inner = SshRemoteEndpointInner::try_parse(src)?;
        Ok(Self { src, inner })
    }

    pub fn to_owned(&self) -> SshRemoteEndpointOwned {
        SshRemoteEndpointOwned {
            src: self.src.to_string(),
            inner: self.inner,
        }
    }

    pub fn get_user(&self) -> &str {
        self.inner.get_user(self.src)
    }

    pub fn get_host(&self) -> &str {
        self.inner.get_host(self.src)
    }

    pub fn get_port(&self) -> Option<&str> {
        self.inner.get_port(self.src)
    }

    pub fn get_host_port(&self) -> (&str, u16) {
        self.inner.get_host_port(self.src)
    }

    pub fn as_str(&self) -> &str {
        self.src
    }
}

pub struct SshRemoteEndpointOwned {
    src: String,
    inner: SshRemoteEndpointInner,
}

impl SshRemoteEndpointOwned {
    pub fn try_parse(src: String) -> Result<Self, String> {
        let inner = SshRemoteEndpointInner::try_parse(src.as_str())?;
        Ok(Self { src, inner })
    }

    pub fn to_ref<'s>(&'s self) -> SshRemoteEndpoint<'s> {
        SshRemoteEndpoint {
            src: self.src.as_str(),
            inner: self.inner,
        }
    }

    pub fn get_user(&self) -> &str {
        self.inner.get_user(&self.src)
    }

    pub fn get_host(&self) -> &str {
        self.inner.get_host(&self.src)
    }

    pub fn get_port(&self) -> Option<&str> {
        self.inner.get_port(&self.src)
    }

    pub fn get_host_port(&self) -> (&str, u16) {
        self.inner.get_host_port(&self.src)
    }

    pub fn as_str(&self) -> &str {
        &self.src
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_full_ssh_string() {
        let ssh = super::SshRemoteEndpoint::try_parse("ssh://user@host:223");
        assert!(ssh.is_ok());
        let ssh = ssh.unwrap();
        assert_eq!(ssh.get_user(), "user");
        assert_eq!(ssh.get_host(), "host");
        assert_eq!(ssh.get_port(), Some("223"));
        assert_eq!(ssh.get_host_port(), ("host", 223));
    }

    #[test]
    fn test_full_ssh_string_with_no_slash() {
        let ssh = super::SshRemoteEndpoint::try_parse("ssh:user@host:222");
        assert!(ssh.is_ok());
        let ssh = ssh.unwrap();
        assert_eq!(ssh.get_user(), "user");
        assert_eq!(ssh.get_host(), "host");
        assert_eq!(ssh.get_port(), Some("222"));
        assert_eq!(ssh.get_host_port(), ("host", 222));
    }

    #[test]
    fn test_full_ssh_string_with_no_port() {
        let ssh = super::SshRemoteEndpoint::try_parse("ssh:user@host");
        assert!(ssh.is_ok());
        let ssh = ssh.unwrap();
        assert_eq!(ssh.get_user(), "user");
        assert_eq!(ssh.get_host(), "host");
        assert_eq!(ssh.get_port(), None);
        assert_eq!(ssh.get_host_port(), ("host", 22));
    }

    #[test]
    fn test_upper_case_scheme() {
        let ssh = super::SshRemoteEndpoint::try_parse("SSH://user@host:22").unwrap();
        assert_eq!(ssh.get_user(), "user");
        assert_eq!(ssh.get_host(), "host");
        assert_eq!(ssh.get_port(), Some("22"));
        assert_eq!(ssh.get_host_port(), ("host", 22));
    }

    #[test]
    fn test_no_scheme_and_no_port() {
        let ssh = super::SshRemoteEndpoint::try_parse("user@host").unwrap();
        assert_eq!(ssh.get_user(), "user");
        assert_eq!(ssh.get_host(), "host");
        assert_eq!(ssh.get_port(), None);
        assert_eq!(ssh.get_host_port(), ("host", 22));
    }

    #[test]
    fn test_no_scheme_with_port() {
        let ssh = super::SshRemoteEndpoint::try_parse("user@host:22").unwrap();
        assert_eq!(ssh.get_user(), "user");
        assert_eq!(ssh.get_host(), "host");
        assert_eq!(ssh.get_port(), Some("22"));
        assert_eq!(ssh.get_host_port(), ("host", 22));
    }

    #[test]
    fn test_user_name_starting_with_ssh() {
        let ssh = super::SshRemoteEndpoint::try_parse("sshuser@host:22").unwrap();
        assert_eq!(ssh.get_user(), "sshuser");
        assert_eq!(ssh.get_host(), "host");
        assert_eq!(ssh.get_port(), Some("22"));
        assert_eq!(ssh.get_host_port(), ("host", 22));
    }

    #[test]
    fn test_scheme_is_exactly_ssh() {
        for src in [
            "http://user@host:22",
            "ssh2://user@host:22",
            "sshx:user@host",
        ] {
            assert!(super::SshRemoteEndpoint::try_parse(src).is_err(), "{src}");
        }
    }

    #[test]
    fn test_extra_colon_with_no_scheme() {
        assert!(super::SshRemoteEndpoint::try_parse("a@b:1:2").is_err());
    }

    #[test]
    fn test_invalid_port() {
        for src in [
            "ssh://user@host:22x",
            "ssh://user@host:99999",
            "ssh://user@host:65536",
            "ssh://user@host:",
            "ssh://user@host:22/path",
            "user@host:22x",
        ] {
            assert!(super::SshRemoteEndpoint::try_parse(src).is_err(), "{src}");
        }
    }

    #[test]
    fn test_max_port() {
        let ssh = super::SshRemoteEndpoint::try_parse("ssh://user@host:65535").unwrap();
        assert_eq!(ssh.get_port(), Some("65535"));
        assert_eq!(ssh.get_host_port(), ("host", 65535));
    }

    #[test]
    fn test_non_ascii_user() {
        let ssh = super::SshRemoteEndpoint::try_parse("ssh://юзер@127.0.0.1:22").unwrap();
        assert_eq!(ssh.get_user(), "юзер");
        assert_eq!(ssh.get_host(), "127.0.0.1");
        assert_eq!(ssh.get_port(), Some("22"));
        assert_eq!(ssh.get_host_port(), ("127.0.0.1", 22));
    }

    #[test]
    fn test_one_non_ascii_char_user() {
        let ssh = super::SshRemoteEndpoint::try_parse("ssh://ю@host").unwrap();
        assert_eq!(ssh.get_user(), "ю");
        assert_eq!(ssh.get_host(), "host");
        assert_eq!(ssh.get_port(), None);
        assert_eq!(ssh.get_host_port(), ("host", 22));
    }

    #[test]
    fn test_non_ascii_host_with_no_port() {
        let ssh = super::SshRemoteEndpoint::try_parse("ssh://user@хост").unwrap();
        assert_eq!(ssh.get_user(), "user");
        assert_eq!(ssh.get_host(), "хост");
        assert_eq!(ssh.get_port(), None);
        assert_eq!(ssh.get_host_port(), ("хост", 22));
    }

    #[test]
    fn test_non_ascii_host_with_port() {
        let ssh = super::SshRemoteEndpoint::try_parse("ssh://user@хост:22").unwrap();
        assert_eq!(ssh.get_user(), "user");
        assert_eq!(ssh.get_host(), "хост");
        assert_eq!(ssh.get_port(), Some("22"));
        assert_eq!(ssh.get_host_port(), ("хост", 22));
    }

    #[test]
    fn test_non_ascii_owned() {
        let ssh =
            super::SshRemoteEndpointOwned::try_parse("ssh://юзер@хост:22".to_string()).unwrap();
        assert_eq!(ssh.get_user(), "юзер");
        assert_eq!(ssh.get_host_port(), ("хост", 22));
        assert_eq!(ssh.to_ref().to_owned().get_host_port(), ("хост", 22));
    }

    #[test]
    fn test_ipv6_host_in_brackets() {
        for src in ["ssh://user@[::1]:22", "ssh://user@[::1]"] {
            assert!(super::SshRemoteEndpoint::try_parse(src).is_err(), "{src}");
        }
    }

    #[test]
    fn test_empty_user() {
        let ssh = super::SshRemoteEndpoint::try_parse("ssh://@host:22").unwrap();
        assert_eq!(ssh.get_user(), "");
        assert_eq!(ssh.get_host(), "host");
        assert_eq!(ssh.get_port(), Some("22"));
        assert_eq!(ssh.get_host_port(), ("host", 22));
    }

    #[test]
    fn test_empty_host() {
        let ssh = super::SshRemoteEndpoint::try_parse("ssh://user@:22").unwrap();
        assert_eq!(ssh.get_user(), "user");
        assert_eq!(ssh.get_host(), "");
        assert_eq!(ssh.get_port(), Some("22"));
        assert_eq!(ssh.get_host_port(), ("", 22));
    }

    #[test]
    fn test_empty_user_and_host() {
        let ssh = super::SshRemoteEndpoint::try_parse("@").unwrap();
        assert_eq!(ssh.get_user(), "");
        assert_eq!(ssh.get_host(), "");
        assert_eq!(ssh.get_port(), None);
        assert_eq!(ssh.get_host_port(), ("", 22));
    }

    #[test]
    fn test_rejected_strings() {
        for src in [
            "ssh://user:pass@host:22",
            "user:pass@host",
            "ssh://host:22",
            "",
        ] {
            assert!(super::SshRemoteEndpoint::try_parse(src).is_err(), "{src}");
        }
    }

    #[test]
    fn test_accepted_string_never_panics_in_getters() {
        let schemes = [
            "", "ssh:", "ssh://", "SSH://", "sshx:", "http://", "//", ":",
        ];
        let users = ["user", "", "ю", "юзер", "a:b", "a@b", "ssh"];
        let hosts = ["host", "", "хост", "[::1]", "127.0.0.1", "a/b"];
        let ports = [
            "", ":", ":22", ":22x", ":99999", ":1:2", ":22/path", ":２２",
        ];

        for scheme in schemes {
            for user in users {
                for user_separator in ["@", ""] {
                    for host in hosts {
                        for port in ports {
                            let src = format!("{scheme}{user}{user_separator}{host}{port}");

                            let Ok(ssh) = super::SshRemoteEndpoint::try_parse(&src) else {
                                continue;
                            };

                            let (user, host) = (ssh.get_user(), ssh.get_host());

                            let (expected_port, parsed) = match ssh.get_port() {
                                Some(port) => {
                                    (port.parse().unwrap(), format!("{user}@{host}:{port}"))
                                }
                                None => (22, format!("{user}@{host}")),
                            };

                            assert_eq!(ssh.get_host_port(), (host, expected_port), "{src}");
                            assert!(src.ends_with(&parsed), "{src}");
                        }
                    }
                }
            }
        }
    }
}
