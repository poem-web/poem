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
///
/// # Examples
///
/// ```
/// use poem::{handler, test::TestClient, Route};
///
/// #[handler]
/// fn index() {}
///
/// let app = Route::new().at("/", index);
///
/// let cli = TestClient::new(index);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// cli.get("/").send().await.assert_status_is_ok();
/// # });
/// ```
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
        /// Create a `GET` request.
        (get, GET),
        /// Create a `POST` request.
        (post, POST),
        /// Create a `PUT` request.
        (put, PUT),
        /// Create a `DELETE` request.
        (delete, DELETE),
        /// Create a `HEAD` request.
        (head, HEAD),
        /// Create a `OPTIONS` request.
        (options, OPTIONS),
        /// Create a `CONNECT` request.
        (connect, CONNECT),
        /// Create a `PATCH` request.
        (patch, PATCH),
        /// Create a `TRACE` request.
        (trace, TRACE)
    );
}
