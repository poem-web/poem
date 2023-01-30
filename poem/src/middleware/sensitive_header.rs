use std::collections::HashSet;

use http::{header::HeaderName, HeaderMap};

use crate::{Endpoint, IntoResponse, Middleware, Request, Response, Result};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
enum AppliedTo {
    RequestOnly,
    ResponseOnly,
    #[default]
    Both,
}

/// Middleware for mark headers value represents sensitive information.
///
/// Sensitive data could represent passwords or other data that should not be
/// stored on disk or in memory. By marking header values as sensitive,
/// components using this crate can be instructed to treat them with special
/// care for security reasons. For example, caches can avoid storing sensitive
/// values, and `HPACK` encoders used by `HTTP/2.0` implementations can choose
/// not to compress them.
///
/// Additionally, sensitive values will be masked by the `Debug` implementation
/// of HeaderValue.
///
/// # Reference
///
/// - <https://docs.rs/http/0.2.6/http/header/struct.HeaderValue.html#method.set_sensitive>
/// - <https://docs.rs/http/0.2.6/http/header/struct.HeaderValue.html#method.is_sensitive>
#[derive(Default)]
pub struct SensitiveHeader {
    headers: HashSet<HeaderName>,
    applied_to: AppliedTo,
}

impl SensitiveHeader {
    /// Create new `SensitiveHeader` middleware.
    #[must_use]
    pub fn new() -> Self {
        Default::default()
    }

    /// Applies to request headers only.
    #[must_use]
    pub fn request_only(self) -> Self {
        Self {
            applied_to: AppliedTo::RequestOnly,
            ..self
        }
    }

    /// Applies to responses headers only.
    #[must_use]
    pub fn response_only(self) -> Self {
        Self {
            applied_to: AppliedTo::ResponseOnly,
            ..self
        }
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

impl<E: Endpoint> Middleware<E> for SensitiveHeader {
    type Output = SensitiveHeaderEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        SensitiveHeaderEndpoint {
            inner: ep,
            headers: self.headers.clone(),
            applied_to: self.applied_to,
        }
    }
}

/// Endpoint for SensitiveHeader middleware.
pub struct SensitiveHeaderEndpoint<E> {
    inner: E,
    headers: HashSet<HeaderName>,
    applied_to: AppliedTo,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for SensitiveHeaderEndpoint<E> {
    type Output = Response;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        if self.applied_to != AppliedTo::ResponseOnly {
            set_sensitive(req.headers_mut(), &self.headers);
        }

        let mut resp = self.inner.call(req).await?.into_response();

        if self.applied_to != AppliedTo::RequestOnly {
            set_sensitive(resp.headers_mut(), &self.headers);
        }

        Ok(resp)
    }
}

#[allow(clippy::mutable_key_type)]
fn set_sensitive(headers: &mut HeaderMap, names: &HashSet<HeaderName>) {
    for name in names {
        if let Some(value) = headers.get_mut(name) {
            value.set_sensitive(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        handler,
        test::{TestClient, TestRequestBuilder},
        EndpointExt,
    };

    fn create_middleware() -> SensitiveHeader {
        SensitiveHeader::new()
            .header("x-api-key1")
            .header("x-api-key2")
            .header("x-api-key3")
            .header("x-api-key4")
    }

    fn create_request<T: Endpoint>(cli: &TestClient<T>) -> TestRequestBuilder<'_, T> {
        cli.get("/")
            .header("x-api-key1", "a")
            .header("x-api-key2", "b")
    }

    #[tokio::test]
    async fn test_sensitive_header_request_only() {
        #[handler(internal)]
        fn index(headers: &HeaderMap) -> impl IntoResponse {
            assert!(headers.get("x-api-key1").unwrap().is_sensitive());
            assert!(headers.get("x-api-key2").unwrap().is_sensitive());

            ().with_header("x-api-key3", "c")
                .with_header("x-api-key4", "c")
        }

        let cli = TestClient::new(index.with(create_middleware().request_only()));

        let resp = create_request(&cli).send().await;
        assert!(!resp.0.headers().get("x-api-key3").unwrap().is_sensitive());
        assert!(!resp.0.headers().get("x-api-key4").unwrap().is_sensitive());
    }

    #[tokio::test]
    async fn test_sensitive_header_response_only() {
        #[handler(internal)]
        fn index(headers: &HeaderMap) -> impl IntoResponse {
            assert!(!headers.get("x-api-key1").unwrap().is_sensitive());
            assert!(!headers.get("x-api-key2").unwrap().is_sensitive());

            ().with_header("x-api-key3", "c")
                .with_header("x-api-key4", "c")
        }

        let cli = TestClient::new(index.with(create_middleware().response_only()));

        let resp = create_request(&cli).send().await;
        assert!(resp.0.headers().get("x-api-key3").unwrap().is_sensitive());
        assert!(resp.0.headers().get("x-api-key4").unwrap().is_sensitive());
    }

    #[tokio::test]
    async fn test_sensitive_header_both() {
        #[handler(internal)]
        fn index(headers: &HeaderMap) -> impl IntoResponse {
            assert!(headers.get("x-api-key1").unwrap().is_sensitive());
            assert!(headers.get("x-api-key2").unwrap().is_sensitive());

            ().with_header("x-api-key3", "c")
                .with_header("x-api-key4", "c")
        }

        let cli = TestClient::new(index.with(create_middleware()));
        let resp = create_request(&cli).send().await;

        assert!(resp.0.headers().get("x-api-key3").unwrap().is_sensitive());
        assert!(resp.0.headers().get("x-api-key4").unwrap().is_sensitive());
    }
}
