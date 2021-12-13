use std::{
    io::Error as IoError,
    ops::{Deref, DerefMut},
};

use bytes::Bytes;
use futures_util::Stream;
use poem::{Body, FromRequest, IntoResponse, Request, RequestBody, Response};
use tokio::io::AsyncRead;

use crate::{
    payload::{ParsePayload, Payload},
    registry::{MetaMediaType, MetaResponse, MetaResponses, MetaSchema, MetaSchemaRef, Registry},
    ApiResponse, ParseRequestError,
};

/// A stream for binary payload.
pub struct BinaryStream(Body);

impl BinaryStream {
    /// Create a [`BinaryStream`].
    pub fn new(reader: impl AsyncRead + Send + 'static) -> Self {
        Self(Body::from_async_read(reader))
    }

    /// Create a body object from bytes stream.
    pub fn from_bytes_stream<S, O, E>(stream: S) -> Self
    where
        S: Stream<Item = Result<O, E>> + Send + 'static,
        O: Into<Bytes> + 'static,
        E: std::error::Error + Send + Sync + 'static,
    {
        Self(Body::from_bytes_stream(stream))
    }

    /// Consumes this object to return a reader.
    pub fn into_async_read(self) -> impl AsyncRead + Unpin + Send + 'static {
        self.0.into_async_read()
    }

    /// Consumes this object to return a bytes stream.
    pub fn into_bytes_stream(self) -> impl Stream<Item = Result<Bytes, IoError>> + Send + 'static {
        self.0.into_bytes_stream()
    }
}

impl From<BinaryStream> for Body {
    fn from(stream: BinaryStream) -> Self {
        stream.0
    }
}

/// A binary payload.
///
/// # Examples
///
/// ```rust
/// use poem::{
///     error::BadRequest,
///     http::{Method, StatusCode, Uri},
///     IntoEndpoint, Request, Result,
/// };
/// use poem_openapi::{
///     payload::{Binary, BinaryStream, Json},
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
///     async fn upload_binary_stream(&self, data: Binary<BinaryStream>) -> Result<Json<usize>> {
///         let mut reader = data.0.into_async_read();
///         let mut bytes = Vec::new();
///         reader.read_to_end(&mut bytes).await.map_err(BadRequest)?;
///         Ok(Json(bytes.len()))
///     }
/// }
///
/// let api = OpenApiService::new(MyApi, "Demo", "0.1.0").into_endpoint();
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = api
///     .call(
///         Request::builder()
///             .method(Method::POST)
///             .content_type("application/octet-stream")
///             .uri(Uri::from_static("/upload"))
///             .body("abcdef"),
///     )
///     .await;
/// assert_eq!(resp.status(), StatusCode::OK);
/// assert_eq!(resp.into_body().into_string().await.unwrap(), "6");
///
/// let resp = api
///     .call(
///         Request::builder()
///             .method(Method::POST)
///             .content_type("application/octet-stream")
///             .uri(Uri::from_static("/upload_stream"))
///             .body("abcdef"),
///     )
///     .await;
/// assert_eq!(resp.status(), StatusCode::OK);
/// assert_eq!(resp.into_body().into_string().await.unwrap(), "6");
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

    async fn from_request(
        request: &Request,
        body: &mut RequestBody,
    ) -> Result<Self, ParseRequestError> {
        Ok(Self(<Vec<u8>>::from_request(request, body).await.map_err(
            |err| ParseRequestError::ParseRequestBody(err.into_response()),
        )?))
    }
}

#[poem::async_trait]
impl ParsePayload for Binary<Bytes> {
    const IS_REQUIRED: bool = true;

    async fn from_request(
        request: &Request,
        body: &mut RequestBody,
    ) -> Result<Self, ParseRequestError> {
        Ok(Self(Bytes::from_request(request, body).await.map_err(
            |err| ParseRequestError::ParseRequestBody(err.into_response()),
        )?))
    }
}

#[poem::async_trait]
impl ParsePayload for Binary<BinaryStream> {
    const IS_REQUIRED: bool = true;

    async fn from_request(
        request: &Request,
        body: &mut RequestBody,
    ) -> Result<Self, ParseRequestError> {
        Ok(Self(
            Body::from_request(request, body)
                .await
                .map(BinaryStream)
                .map_err(|err| ParseRequestError::ParseRequestBody(err.into_response()))?,
        ))
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
impl_apirequest_for_payload!(Binary<BinaryStream>);
