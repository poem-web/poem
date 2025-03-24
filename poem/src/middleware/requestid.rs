use tracing::{Instrument, error, error_span};
use uuid::Uuid;

use crate::{
    Endpoint, Error, FromRequest, IntoResponse, Middleware, Request, Response, Result,
    http::StatusCode,
};

const X_REQUEST_ID: &str = "x-request-id";

/// Whether to use the request ID supplied in the request.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(docsrs, doc(cfg(feature = "requestid")))]
pub enum ReuseId {
    /// Use the incoming request ID.
    Use,
    /// Ignore the incoming request ID and generate a random ID.
    #[default]
    Ignore,
}

/// Middleware to add a unique ID to every incoming request.
#[cfg_attr(docsrs, doc(cfg(feature = "requestid")))]
pub struct RequestId {
    header_name: String,
    use_incoming_id: ReuseId,
}

impl RequestId {
    /// Create a middleware that uses the `x-request-id` for the ID header.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a middleware that uses `header_name` for the ID header.
    #[must_use]
    pub fn with_header_name(header_name: impl AsRef<str>) -> Self {
        Self {
            header_name: header_name.as_ref().to_string(),
            ..Default::default()
        }
    }

    /// Configure whether to use the incoming ID.
    #[must_use]
    pub fn reuse_id(self, reuse_id: ReuseId) -> Self {
        Self {
            use_incoming_id: reuse_id,
            ..self
        }
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self {
            header_name: X_REQUEST_ID.to_string(),
            use_incoming_id: ReuseId::default(),
        }
    }
}

impl<E: Endpoint> Middleware<E> for RequestId {
    type Output = RequestIdEndpoint<E>;
    fn transform(&self, next: E) -> Self::Output {
        RequestIdEndpoint {
            next,
            header_name: self.header_name.clone(),
            use_incoming_id: self.use_incoming_id,
        }
    }
}

/// Endpoint for the `RequestId` middleware.
#[cfg_attr(docsrs, doc(cfg(feature = "requestid")))]
pub struct RequestIdEndpoint<E> {
    next: E,
    header_name: String,
    use_incoming_id: ReuseId,
}

impl<E: Endpoint> Endpoint for RequestIdEndpoint<E> {
    type Output = Response;

    async fn call(&self, mut request: Request) -> Result<Self::Output> {
        let request_id = if self.use_incoming_id == ReuseId::Use {
            request
                .header(&self.header_name)
                .map_or_else(|| Uuid::new_v4().to_string(), ToString::to_string)
        } else {
            Uuid::new_v4().to_string()
        };
        request.set_data(ReqId(request_id.clone()));
        let response = self.next.call(request);
        let response = response.instrument(error_span!("", %request_id));
        match response.await {
            Ok(res) => Ok(res
                .with_header(&self.header_name, request_id.to_string())
                .into_response()),
            Err(e) => Err(e),
        }
    }
}

/// A request ID which can be extracted in handler functions.
#[cfg_attr(docsrs, doc(cfg(feature = "requestid")))]
#[derive(Clone)]
pub struct ReqId(String);

impl std::fmt::Display for ReqId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a> FromRequest<'a> for ReqId {
    async fn from_request(req: &'a Request, _: &mut crate::RequestBody) -> Result<Self> {
        Ok(req
            .extensions()
            .get::<ReqId>()
            .ok_or_else(|| {
                error!("`RequestId` middleware is not active, while trying to extract `ReqId`!");
                Error::from_string(
                    "no associated request_id",
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?
            .clone())
    }
}

impl IntoResponse for ReqId {
    fn into_response(self) -> Response {
        self.to_string().into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EndpointExt, Route, get, handler, test::TestClient};

    #[handler(internal)]
    fn reply_with_req_id(req_id: ReqId) -> ReqId {
        req_id
    }

    fn app(middleware: RequestId) -> impl Endpoint {
        Route::new()
            .at("/", get(reply_with_req_id))
            .with(middleware)
    }

    #[tokio::test]
    async fn x_request_id_header_is_present() {
        let app = app(RequestId::default());
        let cli = TestClient::new(app);
        let response = cli.get("/").send().await;

        response.assert_header_exist(X_REQUEST_ID);
        response.assert_status_is_ok();
    }

    #[tokio::test]
    async fn extracted_id_matches_header() {
        let app = app(RequestId::default());
        let cli = TestClient::new(app);
        let response = cli.get("/").send().await;
        let header_value = response.0.header(X_REQUEST_ID).unwrap().to_string();
        let body_value = response.0.into_body().into_string().await.unwrap();

        assert_eq!(header_value, body_value);
    }

    #[tokio::test]
    async fn custom_header() {
        let header_name = "y-request-id";
        let app = app(RequestId::with_header_name(header_name));
        let cli = TestClient::new(app);
        let mut response = cli.get("/").send().await;
        let header_value = response.0.header(header_name).unwrap().to_string();
        let body_value = response.0.take_body().into_string().await.unwrap();

        response.assert_header_exist(header_name);
        assert_eq!(header_value, body_value);
    }

    #[tokio::test]
    async fn use_incoming_id() {
        let id = "foobar";
        let app = app(RequestId::default().reuse_id(ReuseId::Use));
        let cli = TestClient::new(app);
        let response = cli.get("/").header(X_REQUEST_ID, id).send().await;

        response.assert_header_exist(X_REQUEST_ID);
        assert_eq!(response.0.header(X_REQUEST_ID), Some(id));
    }

    #[tokio::test]
    async fn ignore_incoming_id() {
        let id = "foobar";
        let app = app(RequestId::default().reuse_id(ReuseId::Ignore));
        let cli = TestClient::new(app);
        let response = cli.get("/").header(X_REQUEST_ID, id).send().await;

        response.assert_header_exist(X_REQUEST_ID);
        assert_ne!(response.0.header(X_REQUEST_ID), Some(id));
    }

    #[tokio::test]
    async fn use_incoming_id_different_header() {
        let header_name = "y-request-id";
        let id = "foobar";
        let app = app(RequestId::with_header_name(header_name).reuse_id(ReuseId::Use));
        let cli = TestClient::new(app);
        let response = cli.get("/").header(header_name, id).send().await;

        response.assert_header_exist(header_name);
        assert_eq!(response.0.header(header_name), Some(id));
    }

    #[tokio::test]
    async fn ignore_incoming_id_different_header() {
        let header_name = "y-request-id";
        let id = "foobar";
        let app = app(RequestId::with_header_name(header_name).reuse_id(ReuseId::Ignore));
        let cli = TestClient::new(app);
        let response = cli.get("/").header(header_name, id).send().await;

        response.assert_header_exist(header_name);
        assert_ne!(response.0.header(header_name), Some(id));
    }
}
