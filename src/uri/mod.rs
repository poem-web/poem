//! URI component

mod authority;
mod parts;
mod path_and_query;
mod scheme;

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

pub use authority::Authority;
pub use parts::Parts;
pub use path_and_query::PathAndQuery;
pub use scheme::Scheme;

use crate::error::{Error, ErrorInvalidUri};
use crate::Result;

/// The URI component of a request.
#[derive(Debug, Clone, Hash, Default)]
pub struct Uri(pub(crate) http::Uri);

impl Display for Uri {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Uri {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse::<http::Uri>().map_err(|_| {
            Error::internal_server_error(ErrorInvalidUri)
        })?))
    }
}

impl Uri {
    /// Attempt to convert a [`Uri`] from [`Parts`].
    pub fn from_parts(parts: Parts) -> Result<Self> {
        let mut builder = http::uri::Builder::new();

        if let Some(scheme) = parts.scheme {
            builder = builder.scheme(scheme.0);
        }

        if let Some(authority) = parts.authority {
            builder = builder.authority(authority.0);
        }

        if let Some(path_and_query) = parts.path_and_query {
            builder = builder.path_and_query(path_and_query.0);
        }

        builder
            .build()
            .map_err(|_| Error::internal_server_error(ErrorInvalidUri))
            .map(Self)
    }

    /// Convert a [`Uri`] into [`Parts`].
    pub fn into_parts(self) -> Parts {
        let parts = self.0.into_parts();
        Parts {
            scheme: parts.scheme.map(Scheme),
            authority: parts.authority.map(Authority),
            path_and_query: parts.path_and_query.map(PathAndQuery),
        }
    }

    /// Get the scheme of this [`Uri`].
    pub fn schema(&self) -> Option<Scheme> {
        self.0.scheme().cloned().map(Scheme)
    }

    /// Get the scheme of this [`Uri`] as a `&str`.
    pub fn schema_str(&self) -> Option<&str> {
        self.0.scheme_str()
    }

    /// Get the host of this [`Uri`].
    pub fn host(&self) -> Option<&str> {
        self.0.host()
    }

    /// Get the path of this [`Uri`].
    pub fn path(&self) -> &str {
        self.0.path()
    }

    /// Get the query string of this [`Uri`], starting after the `?`.
    pub fn query(&self) -> Option<&str> {
        self.0.query()
    }

    /// Get the port of this [`Uri`] as a `u16`.
    pub fn port(&self) -> Option<u16> {
        self.0.port_u16()
    }
}
