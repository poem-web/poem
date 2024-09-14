use std::{collections::HashMap, sync::Arc};

use parking_lot::RwLock;

use crate::{error::NotFoundError, Endpoint, IntoResponse, Request, Response, Result};

/// A tokens storage for http01 challenge
#[derive(Debug, Clone, Default)]
pub struct Http01TokensMap(Arc<RwLock<HashMap<String, String>>>);

impl Http01TokensMap {
    /// Create a new http01 challenge tokens storage for use in challenge
    /// endpoint and [`issue_cert`].
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts an entry to the storage
    pub fn insert(&self, token: impl Into<String>, authorization: impl Into<String>) {
        self.0.write().insert(token.into(), authorization.into());
    }

    /// Removes an entry from the storage
    pub fn remove(&self, token: impl AsRef<str>) {
        self.0.write().remove(token.as_ref());
    }

    /// Gets the authorization by token
    pub fn get(&self, token: impl AsRef<str>) -> Option<String> {
        self.0.read().get(token.as_ref()).cloned()
    }
}

/// An endpoint for `HTTP-01` challenge.
pub struct Http01Endpoint {
    /// Challenge keys for http01 domain verification.
    pub keys: Http01TokensMap,
}

impl Endpoint for Http01Endpoint {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        if let Some(token) = req
            .uri()
            .path()
            .strip_prefix("/.well-known/acme-challenge/")
        {
            if let Some(value) = self.keys.get(token) {
                return Ok(value.into_response());
            }
        }

        Err(NotFoundError.into())
    }
}
