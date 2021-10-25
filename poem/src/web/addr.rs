use std::{
    fmt::{self, Display, Formatter},
    ops::Deref,
};

use crate::Addr;

/// Remote peer's address.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RemoteAddr(pub Addr);

impl Deref for RemoteAddr {
    type Target = Addr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for RemoteAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Local server's address.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LocalAddr(pub Addr);

impl Deref for LocalAddr {
    type Target = Addr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for LocalAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
