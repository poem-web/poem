use futures_util::{Stream, StreamExt};
use http::{header, header::HeaderName, HeaderValue, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::{test::json::TestJson, web::sse::Event, Body, Response};

/// A response object for testing.
pub struct TestResponse(Response);

impl TestResponse {
    pub(crate) fn new(resp: Response) -> Self {
        Self(resp)
    }

    /// Consumes this object and returns the [`Response`].
    pub fn into_inner(self) -> Response {
        self.0
    }

    /// Asserts that the status code is equals to `status`.
    pub fn assert_status(&self, status: StatusCode) {
        assert_eq!(self.0.status(), status);
    }

    /// Asserts that the status code is `200 OK`.
    pub fn assert_status_is_ok(&self) {
        self.assert_status(StatusCode::OK);
    }

    /// Asserts that header `key` is equals to `value`.
    pub fn assert_header<K, V>(&self, key: K, value: V)
    where
        K: TryInto<HeaderName>,
        V: TryInto<HeaderValue>,
    {
        let key = key.try_into().map_err(|_| ()).expect("valid header name");
        let value = value
            .try_into()
            .map_err(|_| ())
            .expect("valid header value");

        let value2 = self
            .0
            .headers()
            .get(&key)
            .unwrap_or_else(|| panic!("expect header `{}`", key));

        assert_eq!(value2, value);
    }

    /// Asserts that content type is equals to `content_type`.
    pub fn assert_content_type(&self, content_type: &str) {
        self.assert_header(header::CONTENT_TYPE, content_type);
    }

    /// Consumes this object and return the response body.
    #[inline]
    pub fn into_body(self) -> Body {
        self.0.into_body()
    }

    /// Asserts that the response body is utf8 string and it equals to `text`.
    pub async fn assert_text(self, text: impl AsRef<str>) {
        assert_eq!(
            self.into_body().into_string().await.expect("expect body"),
            text.as_ref()
        );
    }

    /// Asserts that the response body is bytes and it equals to `bytes`.
    pub async fn assert_bytes(self, bytes: impl AsRef<[u8]>) {
        assert_eq!(
            self.into_body().into_vec().await.expect("expect body"),
            bytes.as_ref()
        );
    }

    /// Asserts that the response body is JSON and it equals to `json`.
    pub async fn assert_json(self, json: impl Serialize) {
        assert_eq!(
            self.into_body()
                .into_json::<Value>()
                .await
                .expect("expect body"),
            serde_json::to_value(json).expect("valid json")
        );
    }

    /// Consumes this object and return the [`TestJson`].
    pub async fn json(self) -> TestJson {
        self.into_body()
            .into_json::<TestJson>()
            .await
            .expect("expect body")
    }

    /// Consumes this object and return the SSE events stream.
    pub fn sse_stream(self) -> impl Stream<Item = Event> + Send + Unpin + 'static {
        self.assert_content_type("text/event-stream");
        sse_codec::decode_stream(self.into_body().into_async_read().compat())
            .map(|res| {
                let event = res.expect("valid sse frame");
                match event {
                    sse_codec::Event::Message { id, event, data } => Event::Message {
                        id: id.unwrap_or_default(),
                        event,
                        data,
                    },
                    sse_codec::Event::Retry { retry } => Event::Retry { retry },
                }
            })
            .boxed()
    }

    /// Consumes this object and return the SSE events stream which deserialize
    /// the message data to `T`.
    pub fn typed_sse_stream<T: DeserializeOwned + 'static>(
        self,
    ) -> impl Stream<Item = T> + Send + Unpin + 'static {
        self.sse_stream()
            .filter_map(|event| async move {
                match event {
                    Event::Message { data, .. } => {
                        Some(serde_json::from_str::<T>(&data).expect("valid data"))
                    }
                    Event::Retry { .. } => None,
                }
            })
            .boxed()
    }

    /// Consumes this object and return the SSE events stream which deserialize
    /// the message data to [`TestJson`].
    pub fn json_sse_stream(self) -> impl Stream<Item = TestJson> + Send + Unpin + 'static {
        self.typed_sse_stream::<TestJson>()
    }
}
