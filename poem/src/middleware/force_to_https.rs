use http::{header, uri::Scheme, Uri};

use crate::{web::Redirect, Endpoint, IntoResponse, Middleware, Request, Response, Result};

/// Middleware for force redirect to HTTPS uri.
#[derive(Default)]
pub struct ForceHttps;

impl ForceHttps {
    /// Create new `ForceToHttps` middleware.
    pub fn new() -> Self {
        ForceHttps
    }
}

impl<E> Middleware<E> for ForceHttps
where
    E: Endpoint,
{
    type Output = ForceHttpsEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        ForceHttpsEndpoint { inner: ep }
    }
}

/// Endpoint for ForceToHttps middleware.
pub struct ForceHttpsEndpoint<E> {
    inner: E,
}

#[async_trait::async_trait]
impl<E> Endpoint for ForceHttpsEndpoint<E>
where
    E: Endpoint,
{
    type Output = Response;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        if req.scheme() == &Scheme::HTTP {
            if let Some(host) = req.headers().get(header::HOST).cloned() {
                if let Ok(host) = host.to_str() {
                    let uri_parts = std::mem::take(req.uri_mut()).into_parts();
                    let mut builder = Uri::builder().scheme(Scheme::HTTPS).authority(host);
                    if let Some(path_and_query) = uri_parts.path_and_query {
                        builder = builder.path_and_query(path_and_query);
                    }
                    if let Ok(uri) = builder.build() {
                        return Ok(Redirect::permanent(uri).into_response());
                    }
                }
            }
        }

        self.inner.call(req).await.map(IntoResponse::into_response)
    }
}
