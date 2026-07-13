use super::*;

pub enum RemoteEndpointHostString<'s> {
    Direct(RemoteEndpoint<'s>),
    ViaSsh {
        ssh_remote_host: SshRemoteEndpoint<'s>,
        remote_host_behind_ssh: RemoteEndpoint<'s>,
    },
}

impl<'s> RemoteEndpointHostString<'s> {
    pub fn try_parse(src: &'s str) -> Result<Self, String> {
        let index = src.find("->");

        match index {
            Some(index) => {
                let left_part = &src[..index];
                let right_part = &src[index + 2..];
                let result = Self::ViaSsh {
                    ssh_remote_host: SshRemoteEndpoint::try_parse(left_part)?,
                    remote_host_behind_ssh: RemoteEndpoint::try_parse(right_part)?,
                };

                Ok(result)
            }
            None => {
                let result = RemoteEndpoint::try_parse(src)?;
                Ok(Self::Direct(result))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RemoteEndpointHostString;

    #[test]
    fn test_direct_string() {
        let src = "localhost:8080";

        let string = RemoteEndpointHostString::try_parse(src).unwrap();

        match string {
            RemoteEndpointHostString::Direct(endpoint) => {
                assert_eq!(endpoint.get_host(), "localhost");
                assert_eq!(endpoint.get_port(), Some(8080));
            }
            _ => panic!("Unexpected result"),
        }
    }
    #[test]
    fn test_through_ssh() {
        let src = "ssh://user@remote:223->localhost:8080";

        let string = RemoteEndpointHostString::try_parse(src).unwrap();

        match string {
            RemoteEndpointHostString::ViaSsh {
                ssh_remote_host,
                remote_host_behind_ssh,
            } => {
                assert_eq!(ssh_remote_host.get_host(), "remote");
                assert_eq!(ssh_remote_host.get_port(), Some("223"));
                assert_eq!(remote_host_behind_ssh.get_host(), "localhost");
                assert_eq!(remote_host_behind_ssh.get_port(), Some(8080));
            }
            _ => panic!("Unexpected result"),
        }
    }

    #[test]
    fn test_example_from_real_life() {
        let result =
            RemoteEndpointHostString::try_parse("https://oauth2.googleapis.com/token").unwrap();

        let remote_endpoint = match result {
            RemoteEndpointHostString::Direct(remote_endpoint) => remote_endpoint,
            RemoteEndpointHostString::ViaSsh { .. } => {
                panic!("Unexpected result");
            }
        };

        assert_eq!(remote_endpoint.get_host(), "oauth2.googleapis.com");
        assert!(remote_endpoint.get_scheme().unwrap().is_https());
        assert_eq!(
            remote_endpoint.get_host_port().as_str(),
            "oauth2.googleapis.com:443"
        );

        let owned = remote_endpoint.to_owned();

        assert_eq!(owned.get_host(), "oauth2.googleapis.com");
        assert!(owned.get_scheme().unwrap().is_https());
        assert_eq!(owned.get_host_port().as_str(), "oauth2.googleapis.com:443");
    }

    #[test]
    fn test_unix_socket_via_host_string() {
        // flurl passes `unix:///var/run/docker.sock` straight into
        // RemoteEndpointHostString::try_parse — all spellings must parse and give
        // the same direct unix-socket endpoint.
        for form in [
            "unix:///var/run/docker.sock",
            "unix://var/run/docker.sock",
            "unix+http://var/run/docker.sock",
            "http+unix://var/run/docker.sock",
        ] {
            let parsed = RemoteEndpointHostString::try_parse(form)
                .unwrap_or_else(|e| panic!("{form} should parse, got {e}"));

            match parsed {
                RemoteEndpointHostString::Direct(endpoint) => {
                    assert!(
                        endpoint.get_scheme().unwrap().is_unix_socket(),
                        "{form} must be a unix socket"
                    );
                    assert_eq!(endpoint.get_host(), "/var/run/docker.sock", "host for {form}");
                }
                RemoteEndpointHostString::ViaSsh { .. } => panic!("Unexpected ViaSsh for {form}"),
            }
        }
    }
}
