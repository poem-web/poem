use crate::{
    Endpoint, Middleware, Request, Result, error::SizedLimitError, web::headers::HeaderMapExt,
};

/// Middleware to limit the request payload size.
///
/// If the incoming request does not contain the `Content-Length` header, the
/// middleware will return the `LENGTH_REQUIRED` status code.
///
/// # Errors
///
/// - [`SizedLimitError`]
pub struct SizeLimit {
    max_size: usize,
}

impl SizeLimit {
    /// Create `SizeLimit` middleware.
    pub fn new(max_size: usize) -> Self {
        Self { max_size }
    }
}

impl<E: Endpoint> Middleware<E> for SizeLimit {
    type Output = SizeLimitEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        SizeLimitEndpoint {
            inner: ep,
            max_size: self.max_size,
        }
    }
}

/// Endpoint for the SizeLimit middleware.
pub struct SizeLimitEndpoint<E> {
    inner: E,
    max_size: usize,
}

impl<E: Endpoint> Endpoint for SizeLimitEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let content_length = req
            .headers()
            .typed_get::<headers::ContentLength>()
            .ok_or(SizedLimitError::MissingContentLength)?;

        if content_length.0 as usize > self.max_size {
            return Err(SizedLimitError::PayloadTooLarge.into());
        }

        self.inner.call(req).await
    }
}

#[cfg(test)]
mod tests {
    use http::StatusCode;

    use super::*;
    use crate::{
        endpoint::{EndpointExt, make_sync},
        test::TestClient,
    };

    #[tokio::test]
    async fn size_limit() {
        let ep = make_sync(|_| ()).with(SizeLimit::new(5));
        let cli = TestClient::new(ep);

        cli.post("/")
            .send()
            .await
            .assert_status(StatusCode::LENGTH_REQUIRED);

        cli.post("/")
            .header("content-length", 6)
            .body(&b"123456"[..])
            .send()
            .await
            .assert_status(StatusCode::PAYLOAD_TOO_LARGE);

        cli.post("/")
            .header("content-length", 4)
            .body(&b"1234"[..])
            .send()
            .await
            .assert_status_is_ok();

        cli.post("/")
            .header("content-length", 5)
            .body(&b"12345"[..])
            .send()
            .await
            .assert_status_is_ok();
    }
}
