use poem::{Request, Result};

use crate::{
    auth::ApiKeyAuthorization, base::UrlQuery, error::AuthorizationError, registry::MetaParamIn,
};

/// Used to extract the Api Key from the request.
pub struct ApiKey {
    /// Api key
    pub key: String,
}

impl ApiKeyAuthorization for ApiKey {
    fn from_request(
        req: &Request,
        query: &UrlQuery,
        name: &str,
        in_type: MetaParamIn,
    ) -> Result<Self> {
        match in_type {
            MetaParamIn::Query => query
                .get(name)
                .cloned()
                .map(|value| Self { key: value })
                .ok_or_else(|| AuthorizationError.into()),
            MetaParamIn::Header => req
                .headers()
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(|value| Self {
                    key: value.to_string(),
                })
                .ok_or_else(|| AuthorizationError.into()),
            MetaParamIn::Cookie => req
                .cookie()
                .get(name)
                .as_ref()
                .map(|cookie| Self {
                    key: cookie.value_str().to_string(),
                })
                .ok_or_else(|| AuthorizationError.into()),
            _ => unreachable!(),
        }
    }
}
