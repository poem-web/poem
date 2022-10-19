use std::ops::{Deref, DerefMut};

use bytes::Bytes;
use poem::{Body, FromRequest, IntoResponse, Request, RequestBody, Response, Result};

use crate::{
    payload::{ParsePayload, Payload},
    registry::{MetaMediaType, MetaResponse, MetaResponses, MetaSchema, MetaSchemaRef, Registry},
    ApiResponse,
};

/// A binary payload.
///
/// # Examples
///
/// ```rust
/// use poem::{
///     error::BadRequest,
///     http::{Method, StatusCode, Uri},
///     test::TestClient,
///     Body, IntoEndpoint, Request, Result,
/// };
/// use poem_openapi::{
///     payload::{Binary, Json},
///     OpenApi, OpenApiService,
/// };
/// use tokio::io::AsyncReadExt;
///
/// struct MyApi;
///
/// #[OpenApi]
/// impl MyApi {
///     #[oai(path = "/upload", method = "post")]
///     async fn upload_binary(&self, data: Binary<Vec<u8>>) -> Json<usize> {
///         Json(data.len())
///     }
///
///     #[oai(path = "/upload_stream", method = "post")]
///     async fn upload_binary_stream(&self, data: Binary<Body>) -> Result<Json<usize>> {
///         let mut reader = data.0.into_async_read();
///         let mut bytes = Vec::new();
///         reader.read_to_end(&mut bytes).await.map_err(BadRequest)?;
///         Ok(Json(bytes.len()))
///     }
/// }
///
/// let api = OpenApiService::new(MyApi, "Demo", "0.1.0");
/// let cli = TestClient::new(api);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = cli
///     .post("/upload")
///     .content_type("application/octet-stream")
///     .body("abcdef")
///     .send()
///     .await;
/// resp.assert_status_is_ok();
/// resp.assert_text("6").await;
///
/// let resp = cli
///     .post("/upload_stream")
///     .content_type("application/octet-stream")
///     .body("abcdef")
///     .send()
///     .await;
/// resp.assert_status_is_ok();
/// resp.assert_text("6").await;
/// # });
/// ```
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Binary<T>(pub T);

impl<T> Deref for Binary<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Binary<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Send> Payload for Binary<T> {
    const CONTENT_TYPE: &'static str = "application/octet-stream";

    fn check_content_type(content_type: &str) -> bool {
        matches!(content_type.parse::<mime::Mime>(), Ok(content_type) if content_type.type_() == "application"
                && (content_type.subtype() == "octet-stream"
                || content_type
                    .suffix()
                    .map_or(false, |v| v == "octet-stream")))
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            format: Some("binary"),
            ..MetaSchema::new("string")
        }))
    }
}

#[poem::async_trait]
impl ParsePayload for Binary<Vec<u8>> {
    const IS_REQUIRED: bool = true;

    async fn from_request(request: &Request, body: &mut RequestBody) -> Result<Self> {
        Ok(Self(<Vec<u8>>::from_request(request, body).await?))
    }
}

#[poem::async_trait]
impl ParsePayload for Binary<Bytes> {
    const IS_REQUIRED: bool = true;

    async fn from_request(request: &Request, body: &mut RequestBody) -> Result<Self> {
        Ok(Self(Bytes::from_request(request, body).await?))
    }
}

#[poem::async_trait]
impl ParsePayload for Binary<Body> {
    const IS_REQUIRED: bool = true;

    async fn from_request(request: &Request, body: &mut RequestBody) -> Result<Self> {
        Ok(Self(Body::from_request(request, body).await?))
    }
}

impl<T: Into<Body> + Send> IntoResponse for Binary<T> {
    fn into_response(self) -> Response {
        Response::builder()
            .content_type(Self::CONTENT_TYPE)
            .body(self.0.into())
    }
}

impl<T: Into<Body> + Send> ApiResponse for Binary<T> {
    fn meta() -> MetaResponses {
        MetaResponses {
            responses: vec![MetaResponse {
                description: "",
                status: Some(200),
                content: vec![MetaMediaType {
                    content_type: Self::CONTENT_TYPE,
                    schema: Self::schema_ref(),
                }],
                headers: vec![],
            }],
        }
    }

    fn register(_registry: &mut Registry) {}
}

impl_apirequest_for_payload!(Binary<Vec<u8>>);
impl_apirequest_for_payload!(Binary<Bytes>);
impl_apirequest_for_payload!(Binary<Body>);
