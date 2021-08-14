/// Represents a version of the HTTP spec.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Default)]
pub struct Version(pub(crate) http::Version);

impl Version {
    /// `HTTP/0.9`
    pub const HTTP_09: Version = Version(http::Version::HTTP_09);

    /// `HTTP/1.0`
    pub const HTTP_10: Version = Version(http::Version::HTTP_10);

    /// `HTTP/1.1`
    pub const HTTP_11: Version = Version(http::Version::HTTP_11);

    /// `HTTP/2.0`
    pub const HTTP_2: Version = Version(http::Version::HTTP_2);

    /// `HTTP/3.0`
    pub const HTTP_3: Version = Version(http::Version::HTTP_3);
}
