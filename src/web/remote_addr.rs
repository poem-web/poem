use std::{
    fmt::{self, Display, Formatter},
    sync::Arc,
};

/// Remote peer's address.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct RemoteAddr(Arc<String>);

impl RemoteAddr {
    pub(crate) fn new(addr: impl Display) -> Self {
        Self(Arc::new(addr.to_string()))
    }
}

impl Display for RemoteAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
