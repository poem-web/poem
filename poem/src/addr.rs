use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
};

/// An network address.
#[derive(Debug, Clone)]
pub enum Addr {
    /// Internet socket address
    SocketAddr(std::net::SocketAddr),
    /// Unix domain socket address
    #[cfg(unix)]
    #[cfg_attr(docsrs, doc(cfg(unix)))]
    Unix(std::sync::Arc<tokio::net::unix::SocketAddr>),
    /// Custom address
    Custom(&'static str, Cow<'static, str>),
}

impl Default for Addr {
    fn default() -> Self {
        Self::Custom("unknown", "unknown".into())
    }
}

impl PartialEq for Addr {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Addr::SocketAddr(addr1), Addr::SocketAddr(addr2)) => addr1 == addr2,
            #[cfg(unix)]
            (Addr::Unix(addr1), Addr::Unix(addr2)) => addr1.as_pathname() == addr2.as_pathname(),
            (Addr::Custom(scheme1, addr1), Addr::Custom(scheme2, addr2)) => {
                scheme1 == scheme2 && addr1 == addr2
            }
            _ => false,
        }
    }
}

impl From<std::net::SocketAddr> for Addr {
    fn from(addr: std::net::SocketAddr) -> Self {
        Addr::SocketAddr(addr)
    }
}

#[cfg(unix)]
impl From<tokio::net::unix::SocketAddr> for Addr {
    fn from(addr: tokio::net::unix::SocketAddr) -> Self {
        Addr::Unix(addr.into())
    }
}

impl Addr {
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
    pub fn custom(scheme: &'static str, addr: impl Into<Cow<'static, str>>) -> Self {
        Self::Custom(scheme, addr.into())
    }

    /// If the address is a internet socket address, returns it. Returns None
    /// otherwise.
    pub fn as_socket_addr(&self) -> Option<&std::net::SocketAddr> {
        match self {
            Addr::SocketAddr(addr) => Some(addr),
            _ => None,
        }
    }

    /// If the address is a unix socket address, returns it. Returns None
    /// otherwise.
    #[cfg(unix)]
    #[cfg_attr(docsrs, doc(cfg(unix)))]
    pub fn as_unix_socket_addr(&self) -> Option<&tokio::net::unix::SocketAddr> {
        match self {
            Addr::Unix(addr) => Some(addr),
            _ => None,
        }
    }
}

impl Display for Addr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Addr::SocketAddr(addr) => write!(f, "socket://{addr}"),
            #[cfg(unix)]
            Addr::Unix(addr) => match addr.as_pathname() {
                Some(path) => write!(f, "unix://{}", path.display()),
                None => f.write_str("unix://unknown"),
            },
            Addr::Custom(scheme, addr) => write!(f, "{scheme}://{addr}"),
        }
    }
}
