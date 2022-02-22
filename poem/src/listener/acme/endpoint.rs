use std::{collections::HashMap, sync::Arc};

use parking_lot::RwLock;

use crate::{error::NotFoundError, Endpoint, IntoResponse, Request, Response, Result};

/// An endpoint for `HTTP-01` challenge.
pub struct Http01Endpoint {
    pub(crate) keys: Arc<RwLock<HashMap<String, String>>>,
}

#[async_trait::async_trait]
impl Endpoint for Http01Endpoint {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        if let Some(token) = req
            .uri()
            .path()
            .strip_prefix("/.well-known/acme-challenge/")
        {
            let keys = self.keys.read();
            if let Some(value) = keys.get(token) {
                return Ok(value.clone().into_response());
            }
        }

        Err(NotFoundError.into())
    }
}
