use std::collections::HashSet;

use http::{header::HeaderName, HeaderMap};

use crate::{Endpoint, IntoResponse, Middleware, Request, Response, Result};

/// Middleware for propagate a header from the request to the response.
#[derive(Default)]
pub struct PropagateHeader {
    headers: HashSet<HeaderName>,
}

impl PropagateHeader {
    /// Create new `PropagateHeader` middleware.
    #[must_use]
    pub fn new() -> Self {
        Default::default()
    }

    /// Append a header.
    #[must_use]
    pub fn header<K>(mut self, key: K) -> Self
    where
        K: TryInto<HeaderName>,
    {
        if let Ok(key) = key.try_into() {
            self.headers.insert(key);
        }
        self
    }
}

impl<E: Endpoint> Middleware<E> for PropagateHeader {
    type Output = PropagateHeaderEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        PropagateHeaderEndpoint {
            inner: ep,
            headers: self.headers.clone(),
        }
    }
}

/// Endpoint for PropagateHeader middleware.
pub struct PropagateHeaderEndpoint<E> {
    inner: E,
    headers: HashSet<HeaderName>,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for PropagateHeaderEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let mut headers = HeaderMap::new();

        for header in &self.headers {
            for value in req.headers().get_all(header) {
                headers.append(header.clone(), value.clone());
            }
        }

        let mut resp = self.inner.call(req).await?.into_response();
        resp.headers_mut().extend(headers);
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{handler, EndpointExt};

    #[tokio::test]
    async fn test_propagate_header() {
        #[handler(internal)]
        fn index() {}

        let resp = index
            .with(PropagateHeader::new().header("x-request-id"))
            .call(Request::builder().header("x-request-id", "100").finish())
            .await
            .unwrap();

        assert_eq!(
            resp.headers()
                .get("x-request-id")
                .and_then(|value| value.to_str().ok()),
            Some("100")
        );
    }
}
