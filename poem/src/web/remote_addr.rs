use std::{
    fmt::{self, Display, Formatter},
    sync::Arc,
};

/// Remote peer's address.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RemoteAddr(Arc<str>);

impl RemoteAddr {
    pub(crate) fn new(addr: impl Display) -> Self {
        Self(addr.to_string().into())
    }
}

impl Display for RemoteAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
