use std::collections::HashSet;

use futures_util::{Stream, StreamExt};
use http::{header, header::HeaderName, HeaderValue, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::{test::json::TestJson, web::sse::Event, Response};

/// A response object for testing.
pub struct TestResponse(pub Response);

impl TestResponse {
    pub(crate) fn new(resp: Response) -> Self {
        Self(resp)
    }

    /// Asserts that the status code is equals to `status`.
    pub fn assert_status(&self, status: StatusCode) {
        assert_eq!(self.0.status(), status);
    }

    /// Asserts that the status code is `200 OK`.
    pub fn assert_status_is_ok(&self) {
        self.assert_status(StatusCode::OK);
    }

    /// Asserts that header `key` is not exist.
    pub fn assert_header_is_not_exist<K>(&self, key: K)
    where
        K: TryInto<HeaderName>,
    {
        let key = key.try_into().map_err(|_| ()).expect("valid header name");
        assert!(!self.0.headers().contains_key(key));
    }

    /// Asserts that header `key` exist.
    pub fn assert_header_exist<K>(&self, key: K)
    where
        K: TryInto<HeaderName>,
    {
        let key = key.try_into().map_err(|_| ()).expect("valid header name");
        assert!(self.0.headers().contains_key(key));
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
            .unwrap_or_else(|| panic!("expect header `{key}`"));

        assert_eq!(value2, value);
    }

    /// Asserts that the header `key` is equal to `values` separated by commas.
    pub fn assert_header_csv<K, V, I>(&self, key: K, values: I)
    where
        K: TryInto<HeaderName>,
        V: AsRef<str>,
        I: IntoIterator<Item = V>,
    {
        let expect_values = values.into_iter().collect::<Vec<_>>();
        let expect_values = expect_values
            .iter()
            .map(|value| value.as_ref())
            .collect::<HashSet<_>>();

        let key = key.try_into().map_err(|_| ()).expect("valid header name");
        let value = self
            .0
            .headers()
            .get(&key)
            .unwrap_or_else(|| panic!("expect header `{key}`"));
        let values = value
            .to_str()
            .expect("valid header value")
            .split(',')
            .map(|s| s.trim())
            .collect::<HashSet<_>>();

        assert_eq!(values, expect_values);
    }

    /// Asserts that header `key` is equals to `values`.
    pub fn assert_header_all<K, V, I>(&self, key: K, values: I)
    where
        K: TryInto<HeaderName>,
        V: TryInto<HeaderValue>,
        I: IntoIterator<Item = V>,
    {
        let key = key.try_into().map_err(|_| ()).expect("valid header name");
        let mut values = values
            .into_iter()
            .map(|value| {
                value
                    .try_into()
                    .map_err(|_| ())
                    .expect("valid header value")
            })
            .collect::<Vec<_>>();

        let mut values2 = self
            .0
            .headers()
            .get_all(&key)
            .iter()
            .cloned()
            .collect::<Vec<_>>();

        values.sort();
        values2.sort();
        assert_eq!(values, values2);
    }

    /// Asserts that content type is equals to `content_type`.
    pub fn assert_content_type(&self, content_type: &str) {
        self.assert_header(header::CONTENT_TYPE, content_type);
    }

    /// Asserts that the response body is utf8 string and it equals to `text`.
    pub async fn assert_text(self, text: impl AsRef<str>) {
        assert_eq!(
            self.0.into_body().into_string().await.expect("expect body"),
            text.as_ref()
        );
    }

    /// Asserts that the response body is bytes and it equals to `bytes`.
    pub async fn assert_bytes(self, bytes: impl AsRef<[u8]>) {
        assert_eq!(
            self.0.into_body().into_vec().await.expect("expect body"),
            bytes.as_ref()
        );
    }

    /// Asserts that the response body is JSON and it equals to `json`.
    pub async fn assert_json(self, json: impl Serialize) {
        assert_eq!(
            self.0
                .into_body()
                .into_json::<Value>()
                .await
                .expect("expect body"),
            serde_json::to_value(json).expect("valid json")
        );
    }

    /// Asserts that the response body is XML and it equals to `xml`.
    #[cfg(feature = "xml")]
    pub async fn assert_xml(self, xml: impl Serialize) {
        assert_eq!(
            self.0.into_body().into_string().await.expect("expect body"),
            quick_xml::se::to_string(&xml).expect("valid xml")
        );
    }

    /// Asserts that the response body is XML and it equals to `xml`.
    #[cfg(feature = "yaml")]
    pub async fn assert_yaml(self, yaml: impl Serialize) {
        assert_eq!(
            self.0.into_body().into_string().await.expect("expect body"),
            serde_yaml::to_string(&yaml).expect("valid yaml")
        );
    }

    /// Consumes this object and return the [`TestJson`].
    pub async fn json(self) -> TestJson {
        self.0
            .into_body()
            .into_json::<TestJson>()
            .await
            .expect("expect body")
    }

    /// Consumes this object and return the SSE events stream.
    pub fn sse_stream(self) -> impl Stream<Item = Event> + Send + Unpin + 'static {
        self.assert_content_type("text/event-stream");
        sse_codec::decode_stream(self.0.into_body().into_async_read().compat())
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
