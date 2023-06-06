use http::{header, header::HeaderName, HeaderMap, HeaderValue, Method};

use crate::{test::TestRequestBuilder, Endpoint, IntoEndpoint};

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
pub struct TestClient<E> {
    pub(crate) ep: E,
    pub(crate) default_headers: HeaderMap,
}

impl<E: Endpoint> TestClient<E> {
    /// Create a new client for the specified endpoint.
    pub fn new<T>(ep: T) -> TestClient<T::Endpoint>
    where
        T: IntoEndpoint<Endpoint = E>,
    {
        TestClient {
            ep: ep.into_endpoint(),
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

    /// Upsert on default_headers for the current client.
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
    /// let mut cli = TestClient::new(app).default_header("X-Custom-Header", "test");
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp = cli.get("/").send().await;
    /// resp.assert_status_is_ok();
    /// resp.assert_text("test").await;
    /// # });
    ///
    /// cli.upsert_default_header("X-Custom-Header", "updated");
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp = cli.get("/").send().await;
    /// resp.assert_status_is_ok();
    /// resp.assert_text("updated").await;
    /// # });
    /// ```
    #[must_use]
    pub fn upsert_default_header<K, V>(&mut self, key: K, value: V)
    where
        K: TryInto<HeaderName>,
        V: TryInto<HeaderValue>,
    {
        let key = key.try_into().map_err(|_| ()).expect("valid header name");
        let value = value
            .try_into()
            .map_err(|_| ())
            .expect("valid header value");
        self.default_headers.insert(key, value);
    }

    /// Sets the default content type for each requests.
    #[must_use]
    pub fn default_content_type(self, content_type: impl AsRef<str>) -> Self {
        self.default_header(header::CONTENT_TYPE, content_type.as_ref())
    }

    /// Create a [`TestRequestBuilder`].
    pub fn request(&self, method: Method, uri: impl Into<String>) -> TestRequestBuilder<'_, E> {
        TestRequestBuilder::new(self, method, uri.into())
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
