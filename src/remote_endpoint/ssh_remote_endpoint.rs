use crate::str_utils::StrUtils;

#[derive(Debug, Clone, Copy)]
pub struct SshRemoteEndpointInner {
    user_start: usize,
    user_separator: usize,
    port_separator: Option<usize>,
}

impl SshRemoteEndpointInner {
    pub fn try_parse(src: &str) -> Result<Self, String> {
        let mut user_separator = None;
        let mut first_separator = None;
        let mut second_separator = None;

        let mut pos = 0;
        for c in src.chars() {
            if c == '@' {
                user_separator = Some(pos);
                pos += 1;
                continue;
            }
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

        if user_separator.is_none() {
            return Err(format!("Ssh string is wrong {src}"));
        }

        match first_separator {
            Some(first_separator) => {
                let left_part = &src[..first_separator];

                if left_part.starts_with_case_insensitive("ssh") {
                    match second_separator {
                        Some(second_separator) => {
                            return Ok(Self {
                                user_start: first_separator + 1,
                                user_separator: user_separator.unwrap(),
                                port_separator: Some(second_separator),
                            });
                        }
                        None => {
                            return Ok(Self {
                                user_start: first_separator + 1,
                                user_separator: user_separator.unwrap(),
                                port_separator: None,
                            });
                        }
                    }
                } else {
                    match second_separator {
                        Some(_) => {
                            panic!("Invalid ssh string {src}");
                        }
                        None => {
                            return Ok(Self {
                                user_start: 0,
                                user_separator: user_separator.unwrap(),
                                port_separator: Some(first_separator + 1),
                            });
                        }
                    }
                }
            }
            None => {
                return Ok(Self {
                    user_start: 0,
                    user_separator: user_separator.unwrap(),
                    port_separator: None,
                });
            }
        }
    }

    pub fn get_user<'s>(&self, src: &'s str) -> &'s str {
        let result = &src[self.user_start..self.user_separator];

        if result.starts_with("//") {
            &result[2..]
        } else {
            result
        }
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
        if let Some(port) = self.get_port(src) {
            match port.parse::<u16>() {
                Ok(port) => (self.get_host(src), port),
                Err(_) => panic!("Invalid port {port}"),
            }
        } else {
            (self.get_host(src), 22)
        }
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

    pub fn to_ref(&self) -> SshRemoteEndpoint {
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
}
