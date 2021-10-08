use std::fmt::{self, Display, Formatter};

/// Remote peer's address.
#[derive(Debug, Clone)]
pub enum RemoteAddr {
    /// Internet socket address
    SocketAddr(std::net::SocketAddr),
    /// Unix domain socket address
    #[cfg(unix)]
    #[cfg_attr(docsrs, doc(cfg(unix)))]
    Unix(std::sync::Arc<tokio::net::unix::SocketAddr>),
    /// Custom address
    Custom(&'static str, String),
}

impl PartialEq for RemoteAddr {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RemoteAddr::SocketAddr(addr1), RemoteAddr::SocketAddr(addr2)) => addr1 == addr2,
            #[cfg(unix)]
            (RemoteAddr::Unix(addr1), RemoteAddr::Unix(addr2)) => {
                addr1.as_pathname() == addr2.as_pathname()
            }
            (RemoteAddr::Custom(scheme1, addr1), RemoteAddr::Custom(scheme2, addr2)) => {
                scheme1 == scheme2 && addr1 == addr2
            }
            _ => false,
        }
    }
}

impl From<std::net::SocketAddr> for RemoteAddr {
    fn from(addr: std::net::SocketAddr) -> Self {
        RemoteAddr::SocketAddr(addr)
    }
}

#[cfg(unix)]
impl From<tokio::net::unix::SocketAddr> for RemoteAddr {
    fn from(addr: tokio::net::unix::SocketAddr) -> Self {
        RemoteAddr::Unix(addr.into())
    }
}

impl RemoteAddr {
    /// Create a internet socket address.
    pub fn socket(addr: std::net::SocketAddr) -> Self {
        Self::SocketAddr(addr)
    }

    /// Create a unix socket address.
    #[cfg(unix)]
    #[cfg_attr(docsrs, doc(cfg(unix)))]
    pub fn unix(addr: tokio::net::unix::SocketAddr) -> Self {
        Self::Unix(addr.into())
    }

    /// Create a custom address.
    pub fn custom(scheme: &'static str, addr: impl Into<String>) -> Self {
        Self::Custom(scheme, addr.into())
    }

    /// If the address is a internet socket address, returns it. Returns None
    /// otherwise.
    pub fn as_socket_addr(&self) -> Option<&std::net::SocketAddr> {
        match self {
            RemoteAddr::SocketAddr(addr) => Some(addr),
            _ => None,
        }
    }

    /// If the address is a unix socket address, returns it. Returns None
    /// otherwise.
    #[cfg(unix)]
    #[cfg_attr(docsrs, doc(cfg(unix)))]
    pub fn as_unix_socket_addr(&self) -> Option<&tokio::net::unix::SocketAddr> {
        match self {
            RemoteAddr::Unix(addr) => Some(addr),
            _ => None,
        }
    }
}

impl Display for RemoteAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RemoteAddr::SocketAddr(addr) => write!(f, "socket://{}", addr),
            #[cfg(unix)]
            RemoteAddr::Unix(addr) => match addr.as_pathname() {
                Some(path) => write!(f, "unix://{}", path.display()),
                None => f.write_str("unix://unknown"),
            },
            RemoteAddr::Custom(scheme, addr) => write!(f, "{}://{}", scheme, addr),
        }
    }
}
