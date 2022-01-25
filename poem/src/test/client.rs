use http::{header, header::HeaderName, HeaderMap, HeaderValue, Method};

use crate::{endpoint::BoxEndpoint, test::TestRequestBuilder, Endpoint, Response};

macro_rules! impl_methods {
    ($($(#[$docs:meta])* ($name:ident, $method:ident)),*) => {
        $(
        $(#[$docs])*
        pub fn $name(&self, uri: impl Into<String>) -> TestRequestBuilder<'_, E> {
            TestRequestBuilder::new(self, Method::$method, uri.into())
        }
        )*
    };
}

/// A client for testing.
pub struct TestClient<E = BoxEndpoint<'static, Response>> {
    pub(crate) ep: E,
    pub(crate) default_headers: HeaderMap,
}

impl<E: Endpoint> TestClient<E> {
    /// Create a new client for the specified endpoint.
    pub fn new(ep: E) -> Self {
        Self {
            ep,
            default_headers: Default::default(),
        }
    }

    /// Sets the default header for each requests.
    ///
    /// # Examples
    ///
    /// ```
    /// use poem::{handler, http::HeaderMap, test::TestClient, Route};
    ///
    /// #[handler]
    /// fn index(headers: &HeaderMap) -> String {
    ///     headers
    ///         .get("X-Custom-Header")
    ///         .and_then(|value| value.to_str().ok())
    ///         .unwrap_or_default()
    ///         .to_string()
    /// }
    ///
    /// let app = Route::new().at("/", index);
    /// let cli = TestClient::new(app).default_header("X-Custom-Header", "test");
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp = cli.get("/").send().await;
    /// resp.assert_status_is_ok();
    /// resp.assert_text("test").await;
    /// # });
    /// ```
    #[must_use]
    pub fn default_header<K, V>(mut self, key: K, value: V) -> Self
    where
        K: TryInto<HeaderName>,
        V: TryInto<HeaderValue>,
    {
        let key = key.try_into().map_err(|_| ()).expect("valid header name");
        let value = value
            .try_into()
            .map_err(|_| ())
            .expect("valid header value");
        self.default_headers.append(key, value);
        self
    }

    /// Sets the default content type for each requests.
    #[must_use]
    pub fn default_content_type(self, content_type: impl AsRef<str>) -> Self {
        self.default_header(header::CONTENT_TYPE, content_type.as_ref())
    }

    impl_methods!(
        /// Create a [`TestRequestBuilder`] with `GET` method.
        (get, GET),
        /// Create a [`TestRequestBuilder`] with `POST` method.
        (post, POST),
        /// Create a [`TestRequestBuilder`] with `PUT` method.
        (put, PUT),
        /// Create a [`TestRequestBuilder`] with `DELETE` method.
        (delete, DELETE),
        /// Create a [`TestRequestBuilder`] with `HEAD` method.
        (head, HEAD),
        /// Create a [`TestRequestBuilder`] with `OPTIONS` method.
        (options, OPTIONS),
        /// Create a [`TestRequestBuilder`] with `CONNECT` method.
        (connect, CONNECT),
        /// Create a [`TestRequestBuilder`] with `PATCH` method.
        (patch, PATCH),
        /// Create a [`TestRequestBuilder`] with `TRACE` method.
        (trace, TRACE)
    );
}
