use std::{borrow::Cow, sync::Arc};

use http::{header, uri::Scheme, Uri};

use crate::{web::Redirect, Endpoint, IntoResponse, Middleware, Request, Response, Result};

type FilterFn = Arc<dyn Fn(&Request) -> bool + Send + Sync>;

/// Middleware for force redirect to HTTPS uri.
#[derive(Default)]
pub struct ForceHttps {
    https_port: Option<u16>,
    filter_fn: Option<FilterFn>,
}

impl ForceHttps {
    /// Create new `ForceHttps` middleware.
    pub fn new() -> Self {
        Default::default()
    }

    /// Specify https port.
    #[must_use]
    pub fn https_port(self, port: u16) -> Self {
        Self {
            https_port: Some(port),
            ..self
        }
    }

    /// Uses a closure to determine if a request should be redirect.
    #[must_use]
    pub fn filter(self, predicate: impl Fn(&Request) -> bool + Send + Sync + 'static) -> Self {
        Self {
            filter_fn: Some(Arc::new(predicate)),
            ..self
        }
    }
}

impl<E> Middleware<E> for ForceHttps
where
    E: Endpoint,
{
    type Output = ForceHttpsEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        ForceHttpsEndpoint {
            inner: ep,
            https_port: self.https_port,
            filter_fn: self.filter_fn.clone(),
        }
    }
}

/// Endpoint for ForceHttps middleware.
pub struct ForceHttpsEndpoint<E> {
    inner: E,
    https_port: Option<u16>,
    filter_fn: Option<FilterFn>,
}

#[async_trait::async_trait]
impl<E> Endpoint for ForceHttpsEndpoint<E>
where
    E: Endpoint,
{
    type Output = Response;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        if req.scheme() == &Scheme::HTTP && self.filter_fn.as_ref().map(|f| f(&req)).unwrap_or(true)
        {
            if let Some(host) = req.headers().get(header::HOST).cloned() {
                if let Ok(host) = host.to_str() {
                    let host = redirect_host(host, self.https_port);
                    let uri_parts = std::mem::take(req.uri_mut()).into_parts();
                    let mut builder = Uri::builder().scheme(Scheme::HTTPS).authority(&*host);
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

fn redirect_host(host: &str, https_port: Option<u16>) -> Cow<'_, str> {
    match (host.split_once(':'), https_port) {
        (Some((host, _)), Some(port)) => Cow::Owned(format!("{host}:{port}")),
        (None, Some(port)) => Cow::Owned(format!("{host}:{port}")),
        (_, None) => Cow::Borrowed(host),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redirect_host() {
        assert_eq!(redirect_host("example.com", Some(1234)), "example.com:1234");
        assert_eq!(
            redirect_host("example.com:5678", Some(1234)),
            "example.com:1234"
        );
        assert_eq!(redirect_host("example.com", Some(1234)), "example.com:1234");
        assert_eq!(redirect_host("example.com:1234", None), "example.com:1234");
        assert_eq!(redirect_host("example.com", None), "example.com");
    }
}
