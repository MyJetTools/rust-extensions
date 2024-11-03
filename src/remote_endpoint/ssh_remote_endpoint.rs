use crate::str_utils::StrUtils;

pub struct SshRemoteEndpoint<'s> {
    pub src: &'s str,
    pub user_start: usize,
    pub user_separator: usize,
    pub port_separator: Option<usize>,
}

impl<'s> SshRemoteEndpoint<'s> {
    pub fn try_parse(src: &'s str) -> Result<Self, String> {
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
                                src,
                                user_start: first_separator + 1,
                                user_separator: user_separator.unwrap(),
                                port_separator: Some(second_separator),
                            });
                        }
                        None => {
                            return Ok(Self {
                                src,
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
                                src,
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
                    src,
                    user_start: 0,
                    user_separator: user_separator.unwrap(),
                    port_separator: None,
                });
            }
        }
    }

    pub fn get_user(&self) -> &str {
        let result = &self.src[self.user_start..self.user_separator];

        if result.starts_with("//") {
            &result[2..]
        } else {
            result
        }
    }

    pub fn get_host(&self) -> &str {
        if let Some(port_separator) = self.port_separator {
            &self.src[self.user_separator + 1..port_separator]
        } else {
            &self.src[self.user_separator + 1..]
        }
    }

    pub fn get_port(&self) -> Option<&str> {
        let port_separator = self.port_separator?;
        Some(&self.src[port_separator + 1..])
    }

    pub fn get_host_port(&self) -> (&str, u16) {
        if let Some(port) = self.get_port() {
            match port.parse::<u16>() {
                Ok(port) => (self.get_host(), port),
                Err(_) => panic!("Invalid port {port}"),
            }
        } else {
            (self.get_host(), 22)
        }
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
