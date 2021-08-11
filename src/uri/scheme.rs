use std::fmt::{self, Display, Formatter};

/// Represents the scheme component of a URI
#[derive(Debug, Clone)]
pub struct Scheme(pub(crate) http::uri::Scheme);

impl Scheme {
    /// HTTP protocol scheme
    pub const HTTP: Scheme = Scheme(http::uri::Scheme::HTTP);

    /// HTTP protocol over TLS.
    pub const HTTPS: Scheme = Scheme(http::uri::Scheme::HTTPS);
}

impl Display for Scheme {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
