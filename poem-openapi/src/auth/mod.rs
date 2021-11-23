//! Some certificate types for security scheme.

mod api_key;
mod basic;
mod bearer;

use poem::Request;

pub use self::{api_key::ApiKey, basic::Basic, bearer::Bearer};
use crate::{base::UrlQuery, registry::MetaParamIn, ParseRequestError};

/// Represents a basic authorization extractor.
pub trait BasicAuthorization: Sized {
    /// Extract from the HTTP request.
    fn from_request(req: &Request) -> Result<Self, ParseRequestError>;
}

/// Represents a bearer authorization extractor.
pub trait BearerAuthorization: Sized {
    /// Extract from the HTTP request.
    fn from_request(req: &Request) -> Result<Self, ParseRequestError>;
}

/// Represents an api key authorization extractor.
pub trait ApiKeyAuthorization: Sized {
    /// Extract from the HTTP request.
    fn from_request(
        req: &Request,
        query: &UrlQuery,
        name: &str,
        in_type: MetaParamIn,
    ) -> Result<Self, ParseRequestError>;
}
