use crate::{str_utils::StrUtils, ShortString};

#[derive(Debug, Clone, Copy)]
pub struct HostEndpoint<'s> {
    pub scheme: Option<&'s str>,
    pub host: &'s str,
    pub port: Option<u16>,
}

impl<'s> HostEndpoint<'s> {
    pub fn new(src: &'s str) -> Option<Self> {
        let mut left = None;
        let mut mid = None;
        let mut right = None;

        for itm in src.split(":") {
            if left.is_none() {
                left = Some(itm);
                continue;
            }
            if mid.is_none() {
                mid = Some(itm);
                continue;
            }
            if right.is_none() {
                right = Some(itm);
                continue;
            }
            return None;
        }

        let scheme = if src.starts_with_case_insensitive("http://")
            || src.starts_with_case_insensitive("https://")
        {
            left
        } else {
            None
        };

        let (host, port) = if scheme.is_none() {
            let port = if let Some(mid) = mid {
                mid.parse().ok()
            } else {
                None
            };
            (left?, port)
        } else {
            let port = if let Some(right) = right {
                right.parse().ok()
            } else {
                None
            };
            (mid?, port)
        };

        Some(Self {
            scheme,
            host: if host.starts_with("//") {
                &host[2..]
            } else {
                host
            },
            port,
        })
    }

    pub fn get_host_port(&self, default_port: Option<u64>) -> ShortString {
        let mut result = ShortString::new_empty();
        result.push_str(self.host);
        if let Some(port) = self.port {
            result.push_str(":");
            result.push_str(port.to_string().as_str());
        } else if let Some(default_port) = default_port {
            result.push_str(":");
            result.push_str(default_port.to_string().as_str());
        }
        result
    }

    pub fn get_standard_port(&self) -> u16 {
        if let Some(port) = self.port {
            return port;
        }

        if let Some(scheme) = self.scheme {
            match scheme {
                "https" => return 443,
                "ssh" => return 22,
                "http" => return 80,
                _ => {}
            }
        }

        panic!(
            "Unknown scheme {:?}. Can not assigned default port",
            self.scheme
        );
    }
}

#[cfg(test)]
mod test {
    use super::HostEndpoint;

    #[test]
    fn test_http_with_port() {
        let result = HostEndpoint::new("http://localhost:8000").unwrap();

        assert_eq!(result.scheme, Some("http"));
        assert_eq!(result.host, "localhost");
        assert_eq!(result.port, Some(8000));
    }

    #[test]
    fn test_http_with_no_port() {
        let result = HostEndpoint::new("http://localhost").unwrap();

        assert_eq!(result.scheme, Some("http"));
        assert_eq!(result.host, "localhost");
        assert_eq!(result.port, None);
    }

    #[test]
    fn test_no_scheme_but_has_port() {
        let result = HostEndpoint::new("localhost:8888").unwrap();

        assert_eq!(result.scheme, None);
        assert_eq!(result.host, "localhost");
        assert_eq!(result.port, Some(8888));
    }

    #[test]
    fn test_no_scheme_and_no_port() {
        let result = HostEndpoint::new("localhost").unwrap();

        assert_eq!(result.scheme, None);
        assert_eq!(result.host, "localhost");
        assert_eq!(result.port, None);
    }

    #[test]
    fn test_get_host_port_with_default_port() {
        let result = HostEndpoint::new("localhost").unwrap();

        let host_port = result.get_host_port(Some(80));

        assert_eq!(host_port.as_str(), "localhost:80");
    }
}
