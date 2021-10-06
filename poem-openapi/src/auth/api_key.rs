use std::collections::HashMap;

use poem::Request;

use crate::{auth::ApiKeyAuthorization, registry::MetaParamIn, ParseRequestError};

/// Used to extract the Api Key from the request.
pub struct ApiKey {
    /// Api key
    pub key: String,
}

impl ApiKeyAuthorization for ApiKey {
    fn from_request(
        req: &Request,
        query: &HashMap<String, String>,
        name: &str,
        in_type: MetaParamIn,
    ) -> Result<Self, ParseRequestError> {
        match in_type {
            MetaParamIn::Query => query
                .get(name)
                .cloned()
                .map(|value| Self { key: value })
                .ok_or(ParseRequestError::Authorization),
            MetaParamIn::Header => req
                .headers()
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(|value| Self {
                    key: value.to_string(),
                })
                .ok_or(ParseRequestError::Authorization),
            MetaParamIn::Cookie => req
                .cookie()
                .get(name)
                .as_ref()
                .map(|cookie| Self {
                    key: cookie.value_str().to_string(),
                })
                .ok_or(ParseRequestError::Authorization),
            _ => unreachable!(),
        }
    }
}
