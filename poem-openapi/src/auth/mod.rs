//! Some certificate types for security scheme.

mod api_key;
mod basic;
mod bearer;

use poem::{Request, Result};

pub use self::{api_key::ApiKey, basic::Basic, bearer::Bearer};
use crate::{base::UrlQuery, error::AuthorizationError, registry::MetaParamIn};

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

/// Facilitates the conversion of `Option` into `Results`, for `SecuritySchema` checker.
#[doc(hidden)]
pub enum CheckerReturn<T> {
    Result(Result<T>),
    Option(Option<T>),
}

impl<T> CheckerReturn<T> {
    pub fn into_result(self) -> Result<T> {
        match self {
            Self::Result(result) => result,
            Self::Option(option) => option.ok_or(AuthorizationError.into()),
        }
    }
}

impl<T> From<poem::Result<T>> for CheckerReturn<T> {
    #[inline]
    fn from(result: Result<T>) -> Self {
        Self::Result(result)
    }
}

impl<T> From<Option<T>> for CheckerReturn<T> {
    #[inline]
    fn from(option: Option<T>) -> Self {
        Self::Option(option)
    }
}
