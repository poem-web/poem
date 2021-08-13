use crate::uri::{Authority, PathAndQuery, Scheme};

/// The various parts of a URI.
///
/// This struct is used to provide to and retrieve from a URI.
#[derive(Debug, Clone)]
pub struct Parts {
    /// The scheme component of a URI
    pub scheme: Option<Scheme>,

    /// The authority component of a URI
    pub authority: Option<Authority>,

    /// The origin-form component of a URI
    pub path_and_query: Option<PathAndQuery>,
}
