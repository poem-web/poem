use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use crate::error::{Error, ErrorInvalidUri};

/// Represents the authority component of a URI.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Authority(pub(crate) http::uri::Authority);

impl Authority {
    /// Attempt to convert an [`Authority`] from a static string.
    ///
    /// This function will not perform any copying, and the string will be checked if it is empty or contains an invalid character.
    ///
    /// # Panics
    ///
    /// This function panics if the argument contains invalid characters or is empty.
    #[inline]
    pub fn from_static(src: &'static str) -> Self {
        Authority(http::uri::Authority::from_static(src))
    }

    /// Return a str representation of the authority.
    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Get the host of this [`Authority`].
    #[inline]
    pub fn host(&self) -> &str {
        self.0.host()
    }

    /// Get the port of this [`Authority`].
    #[inline]
    pub fn port(&self) -> Option<u16> {
        self.0.port_u16()
    }
}

impl FromStr for Authority {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Authority(s.parse().map_err(|_| {
            Error::internal_server_error(ErrorInvalidUri)
        })?))
    }
}

impl Display for Authority {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
