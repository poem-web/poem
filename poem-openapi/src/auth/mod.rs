//! Some certificate types for security scheme.

mod api_key;
mod basic;
mod bearer;

use std::collections::HashMap;

pub use api_key::ApiKey;
pub use basic::Basic;
pub use bearer::Bearer;
use poem::Request;

use crate::{registry::MetaParamIn, ParseRequestError};

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
        query: &HashMap<String, String>,
        name: &str,
        in_type: MetaParamIn,
    ) -> Result<Self, ParseRequestError>;
}
