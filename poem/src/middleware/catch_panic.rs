use std::{any::Any, panic::AssertUnwindSafe};

use futures_util::FutureExt;
use http::StatusCode;

use crate::{Endpoint, IntoResponse, Middleware, Request, Response, Result};

/// Panics handler
pub trait PanicHandler: Clone + Sync + Send + 'static {
    /// Response type
    type Response: IntoResponse;

    /// Call this method to create a response when a panic occurs.
    fn get_response(&self, err: Box<dyn Any + Send + 'static>) -> Self::Response;
}

impl PanicHandler for () {
    type Response = (StatusCode, &'static str);

    fn get_response(&self, _err: Box<dyn Any + Send + 'static>) -> Self::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
    }
}

impl<F, R> PanicHandler for F
where
    F: Fn(Box<dyn Any + Send + 'static>) -> R + Send + Sync + Clone + 'static,
    R: IntoResponse,
{
    type Response = R;

    fn get_response(&self, err: Box<dyn Any + Send + 'static>) -> Self::Response {
        (self)(err)
    }
}

/// Middleware that catches panics and converts them into `500 INTERNAL SERVER
/// ERROR` responses.
///
/// # Example
///
/// ```rust
/// use http::StatusCode;
/// use poem::{handler, middleware::CatchPanic, test::TestClient, EndpointExt, Route};
///
/// #[handler]
/// async fn index() {
///     panic!()
/// }
///
/// let app = Route::new().at("/", index).with(CatchPanic::new());
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let cli = TestClient::new(app);
/// let resp = cli.get("/").send().await;
/// resp.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
/// # });
/// ```
pub struct CatchPanic<H> {
    panic_handler: H,
}

impl CatchPanic<()> {
    /// Create new `CatchPanic` middleware.
    #[inline]
    pub fn new() -> Self {
        CatchPanic { panic_handler: () }
    }
}

impl Default for CatchPanic<()> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<H> CatchPanic<H> {
    /// Specifies a panic handler to be used to create a custom response when
    /// a panic occurs.
    ///
    /// # Example
    ///
    /// ```rust
    /// use http::StatusCode;
    /// use poem::{
    ///     handler, middleware::CatchPanic, test::TestClient, EndpointExt, IntoResponse, Route,
    /// };
    ///
    /// #[handler]
    /// async fn index() {
    ///     panic!()
    /// }
    ///
    /// let app = Route::new().at("/", index).with(
    ///     CatchPanic::new().with_handler(|_| "error!".with_status(StatusCode::INTERNAL_SERVER_ERROR)),
    /// );
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let cli = TestClient::new(app);
    /// let resp = cli.get("/").send().await;
    /// resp.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
    /// resp.assert_text("error!").await;
    /// # });
    /// ```
    #[inline]
    pub fn with_handler<T: PanicHandler>(self, handler: T) -> CatchPanic<T> {
        CatchPanic {
            panic_handler: handler,
        }
    }
}

impl<E: Endpoint, H: PanicHandler> Middleware<E> for CatchPanic<H> {
    type Output = CatchPanicEndpoint<E, H>;

    fn transform(&self, ep: E) -> Self::Output {
        CatchPanicEndpoint {
            inner: ep,
            panic_handler: self.panic_handler.clone(),
        }
    }
}

/// Endpoint for the `PanicHandler` middleware.
pub struct CatchPanicEndpoint<E, H> {
    inner: E,
    panic_handler: H,
}

impl<E: Endpoint, H: PanicHandler> Endpoint for CatchPanicEndpoint<E, H> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        match AssertUnwindSafe(self.inner.call(req)).catch_unwind().await {
            Ok(resp) => resp.map(IntoResponse::into_response),
            Err(err) => Ok(self.panic_handler.get_response(err).into_response()),
        }
    }
}
