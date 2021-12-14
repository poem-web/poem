//! Some certificate types for security scheme.

mod api_key;
mod basic;
mod bearer;

use poem::{Request, Result};

pub use self::{api_key::ApiKey, basic::Basic, bearer::Bearer};
use crate::{base::UrlQuery, registry::MetaParamIn};

/// Represents a basic authorization extractor.
pub trait BasicAuthorization: Sized {
    /// Extract from the HTTP request.
    fn from_request(req: &Request) -> Result<Self>;
}

/// Represents a bearer authorization extractor.
pub trait BearerAuthorization: Sized {
    /// Extract from the HTTP request.
    fn from_request(req: &Request) -> Result<Self>;
}

/// Represents an api key authorization extractor.
pub trait ApiKeyAuthorization: Sized {
    /// Extract from the HTTP request.
    fn from_request(
        req: &Request,
        query: &UrlQuery,
        name: &str,
        in_type: MetaParamIn,
    ) -> Result<Self>;
}
