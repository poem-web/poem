use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use crate::error::{Error, ErrorInvalidUri};

/// Represents the path component of a URI
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PathAndQuery(pub(crate) http::uri::PathAndQuery);

impl PathAndQuery {
    /// Convert a [`PathAndQuery`] from a static string.
    ///
    /// This function will not perform any copying, however the string is checked to ensure that it is valid.
    ///
    /// # Panics
    ///
    /// This function panics if the argument is an invalid path and query.
    #[inline]
    pub fn from_static(src: &'static str) -> Self {
        Self(http::uri::PathAndQuery::from_static(src))
    }

    /// Returns the path and query as a string component.
    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Returns the path component.
    #[inline]
    pub fn path(&self) -> &str {
        self.0.path()
    }

    /// Returns the query string component.
    #[inline]
    pub fn query(&self) -> Option<&str> {
        self.0.query()
    }
}

impl FromStr for PathAndQuery {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(PathAndQuery(s.parse().map_err(|_| {
            Error::internal_server_error(ErrorInvalidUri)
        })?))
    }
}

impl Display for PathAndQuery {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
