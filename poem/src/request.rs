use std::{
    fmt::{self, Debug, Formatter},
    future::Future,
    io::Error,
    pin::Pin,
    str::FromStr,
    task::{Context, Poll},
};

use http::uri::Scheme;
use http_body_util::BodyExt;
use hyper::{body::Incoming, rt::Write as _};
use parking_lot::Mutex;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

#[cfg(feature = "cookie")]
use crate::web::cookie::CookieJar;
use crate::{
    RequestBody,
    body::{Body, BoxBody},
    error::{ParsePathError, ParseQueryError, UpgradeError},
    http::{
        Extensions, Method, Uri, Version,
        header::{self, HeaderMap, HeaderName, HeaderValue},
    },
    route::PathParams,
    web::{
        LocalAddr, PathDeserializer, RemoteAddr,
        headers::{Header, HeaderMapExt},
    },
};

pub(crate) struct RequestState {
    pub(crate) local_addr: LocalAddr,
    pub(crate) remote_addr: RemoteAddr,
    pub(crate) scheme: Scheme,
    pub(crate) original_uri: Uri,
    pub(crate) match_params: PathParams,
    #[cfg(feature = "cookie")]
    pub(crate) cookie_jar: Option<CookieJar>,
    pub(crate) on_upgrade: Mutex<Option<OnUpgrade>>,
}

impl Default for RequestState {
    fn default() -> Self {
        Self {
            local_addr: Default::default(),
            remote_addr: Default::default(),
            scheme: Scheme::HTTP,
            original_uri: Default::default(),
            match_params: vec![],
            #[cfg(feature = "cookie")]
            cookie_jar: None,
            on_upgrade: Default::default(),
        }
    }
}

/// Component parts of an HTTP Request.
///
/// The HTTP request head consists of a method, uri, version, and a set of
/// header fields.
pub struct RequestParts {
    /// The request’s method
    pub method: Method,
    /// The request’s URI
    pub uri: Uri,
    /// The request’s version
    pub version: Version,
    /// The request’s headers
    pub headers: HeaderMap,
    /// The request’s extensions
    pub extensions: Extensions,
    pub(crate) state: RequestState,
}

impl Debug for RequestParts {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestParts")
            .field("method", &self.method)
            .field("uri", &self.uri)
            .field("version", &self.version)
            .field("headers", &self.headers)
            .finish()
    }
}

/// Represents an HTTP request.
#[derive(Default)]
pub struct Request {
    method: Method,
    uri: Uri,
    version: Version,
    headers: HeaderMap,
    extensions: Extensions,
    body: Body,
    state: RequestState,
}

impl Debug for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("method", &self.method)
            .field("uri", &self.uri)
            .field("version", &self.version)
            .field("headers", &self.headers)
            .finish()
    }
}

impl From<(http::Request<Incoming>, LocalAddr, RemoteAddr, Scheme)> for Request {
    fn from(
        (req, local_addr, remote_addr, scheme): (
            http::Request<Incoming>,
            LocalAddr,
            RemoteAddr,
            Scheme,
        ),
    ) -> Self {
        let (mut parts, body) = req.into_parts();
        let on_upgrade = Mutex::new(
            parts
                .extensions
                .remove::<hyper::upgrade::OnUpgrade>()
                .map(|fut| OnUpgrade { fut }),
        );

        Self {
            method: parts.method,
            uri: parts.uri.clone(),
            version: parts.version,
            headers: parts.headers,
            extensions: parts.extensions,
            body: Body(body.map_err(Error::other).boxed()),
            state: RequestState {
                local_addr,
                remote_addr,
                scheme,
                original_uri: parts.uri,
                match_params: Default::default(),
                #[cfg(feature = "cookie")]
                cookie_jar: None,
                on_upgrade,
            },
        }
    }
}

impl From<Request> for hyper::Request<BoxBody> {
    fn from(req: Request) -> Self {
        let mut hyper_req = http::Request::builder()
            .method(req.method)
            .uri(req.uri)
            .version(req.version)
            .body(req.body.0)
            .unwrap();
        *hyper_req.headers_mut() = req.headers;
        *hyper_req.extensions_mut() = req.extensions;
        hyper_req
    }
}

impl Request {
    /// Creates a new `Request` with the given components parts and body.
    pub fn from_parts(parts: RequestParts, body: Body) -> Self {
        Self {
            method: parts.method,
            uri: parts.uri,
            version: parts.version,
            headers: parts.headers,
            extensions: parts.extensions,
            body,
            state: parts.state,
        }
    }

    /// Creates a request builder.
    pub fn builder() -> RequestBuilder {
        RequestBuilder {
            method: Method::GET,
            uri: Default::default(),
            version: Default::default(),
            headers: Default::default(),
            extensions: Default::default(),
        }
    }

    /// Returns a reference to the associated HTTP method.
    #[inline]
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Sets the HTTP method for this request.
    #[inline]
    pub fn set_method(&mut self, method: Method) {
        self.method = method;
    }

    /// Returns a reference to the associated URI.
    #[inline]
    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    /// Returns a mutable reference to the associated URI.
    #[inline]
    pub fn uri_mut(&mut self) -> &mut Uri {
        &mut self.uri
    }

    /// Returns a reference to the associated original URI.
    #[inline]
    pub fn original_uri(&self) -> &Uri {
        &self.state.original_uri
    }

    /// Returns the associated version.
    #[inline]
    pub fn version(&self) -> Version {
        self.version
    }

    /// Sets the version for this request.
    #[inline]
    pub fn set_version(&mut self, version: Version) {
        self.version = version;
    }

    /// Returns the scheme of incoming request.
    #[inline]
    pub fn scheme(&self) -> &Scheme {
        &self.state.scheme
    }

    /// Returns a reference to the associated header map.
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Returns a mutable reference to the associated header map.
    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    /// Returns the string value of the specified header.
    ///
    /// NOTE: Returns `None` if the header value is not a valid UTF8 string.
    pub fn header(&self, name: impl AsRef<str>) -> Option<&str> {
        self.headers
            .get(name.as_ref())
            .and_then(|value| value.to_str().ok())
    }

    /// Returns the raw path parameter with the specified `name`.
    pub fn raw_path_param(&self, name: &str) -> Option<&str> {
        self.state
            .match_params
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// Deserialize path parameters.
    ///
    /// See also [`Path`](crate::web::Path)
    ///
    /// # Example
    ///
    /// ```rust
    /// use poem::{
    ///     Endpoint, Request, Result, Route, handler,
    ///     http::{StatusCode, Uri},
    ///     test::TestClient,
    /// };
    ///
    /// #[handler]
    /// fn index(req: &Request) -> Result<String> {
    ///     let (a, b) = req.path_params::<(i32, String)>()?;
    ///     Ok(format!("{}:{}", a, b))
    /// }
    ///
    /// let app = Route::new().at("/:a/:b", index);
    /// let cli = TestClient::new(app);
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp = cli.get("/100/abc").send().await;
    /// resp.assert_status_is_ok();
    /// resp.assert_text("100:abc").await;
    /// # });
    /// ```
    pub fn path_params<T: DeserializeOwned>(&self) -> Result<T, ParsePathError> {
        T::deserialize(PathDeserializer::new(&self.state().match_params))
            .map_err(|_| ParsePathError)
    }

    /// Deserialize query parameters.
    ///
    /// See also [`Query`](crate::web::Query)
    ///
    /// # Example
    ///
    /// ```rust
    /// use poem::{
    ///     Endpoint, Request, Result, Route, handler,
    ///     http::{StatusCode, Uri},
    ///     test::TestClient,
    /// };
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct Params {
    ///     a: i32,
    ///     b: String,
    /// }
    ///
    /// #[handler]
    /// fn index(req: &Request) -> Result<String> {
    ///     let params = req.params::<Params>()?;
    ///     Ok(format!("{}:{}", params.a, params.b))
    /// }
    ///
    /// let app = Route::new().at("/", index);
    /// let cli = TestClient::new(app);
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp = cli
    ///     .get("/")
    ///     .query("a", &100)
    ///     .query("b", &"abc")
    ///     .send()
    ///     .await;
    /// resp.assert_status_is_ok();
    /// resp.assert_text("100:abc").await;
    /// # });
    /// ```
    pub fn params<T: DeserializeOwned>(&self) -> Result<T, ParseQueryError> {
        Ok(serde_urlencoded::from_str(
            self.uri().query().unwrap_or_default(),
        )?)
    }

    /// Returns the content type of this request.
    pub fn content_type(&self) -> Option<&str> {
        self.headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
    }

    /// Returns a reference to the associated extensions.
    #[inline]
    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    /// Returns a mutable reference to the associated extensions.
    #[inline]
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }

    /// Get a reference from extensions, similar to `self.extensions().get()`.
    #[inline]
    pub fn data<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.extensions.get()
    }

    /// Inserts a value to extensions, similar to
    /// `self.extensions().insert(data)`.
    #[inline]
    pub fn set_data(&mut self, data: impl Clone + Send + Sync + 'static) {
        self.extensions.insert(data);
    }

    /// Returns a reference to the remote address.
    #[inline]
    pub fn remote_addr(&self) -> &RemoteAddr {
        &self.state.remote_addr
    }

    /// Returns a reference to the local address.
    #[inline]
    pub fn local_addr(&self) -> &LocalAddr {
        &self.state.local_addr
    }

    /// Returns a reference to the [`CookieJar`]
    #[cfg(feature = "cookie")]
    #[cfg_attr(docsrs, doc(cfg(feature = "cookie")))]
    #[inline]
    pub fn cookie(&self) -> &CookieJar {
        self.state.cookie_jar.as_ref().expect(
            "To use the `CookieJar` extractor, the `CookieJarManager` middleware is required.",
        )
    }

    /// Sets the body for this request.
    pub fn set_body(&mut self, body: impl Into<Body>) {
        self.body = body.into();
    }

    /// Take the body from this request and sets the body to empty.
    #[inline]
    pub fn take_body(&mut self) -> Body {
        std::mem::take(&mut self.body)
    }

    /// Consume this request and return its inner body.
    #[inline]
    pub fn into_body(self) -> Body {
        self.body
    }

    #[inline]
    pub(crate) fn state(&self) -> &RequestState {
        &self.state
    }

    #[inline]
    pub(crate) fn state_mut(&mut self) -> &mut RequestState {
        &mut self.state
    }

    /// Returns the parameters used by the extractor.
    pub fn split(mut self) -> (Request, RequestBody) {
        let body = self.take_body();
        (self, RequestBody::new(body))
    }

    /// Consumes the request returning the head and body parts.
    pub fn into_parts(self) -> (RequestParts, Body) {
        (
            RequestParts {
                method: self.method,
                uri: self.uri,
                version: self.version,
                headers: self.headers,
                extensions: self.extensions,
                state: self.state,
            },
            self.body,
        )
    }

    /// Upgrade the connection and return a stream.
    pub fn take_upgrade(&self) -> Result<OnUpgrade, UpgradeError> {
        self.state
            .on_upgrade
            .lock()
            .take()
            .ok_or(UpgradeError::NoUpgrade)
    }
}

pin_project_lite::pin_project! {
    /// A future for a possible HTTP upgrade.
    pub struct OnUpgrade {
        #[pin] fut: hyper::upgrade::OnUpgrade,
    }
}

impl Future for OnUpgrade {
    type Output = Result<Upgraded, UpgradeError>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.fut
            .poll(cx)
            .map_ok(|stream| Upgraded { stream })
            .map_err(|err| UpgradeError::Other(err.to_string()))
    }
}

pin_project_lite::pin_project! {
    /// An upgraded HTTP connection.
    pub struct Upgraded {
        #[pin] stream: hyper::upgrade::Upgraded,
    }
}

impl AsyncRead for Upgraded {
    #[inline]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut hyper_util::rt::TokioIo::new(self.project().stream)).poll_read(cx, buf)
    }
}

impl AsyncWrite for Upgraded {
    #[inline]
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        self.project().stream.poll_write(cx, buf)
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.project().stream.poll_flush(cx)
    }

    #[inline]
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.project().stream.poll_shutdown(cx)
    }
}

/// An request builder.
pub struct RequestBuilder {
    method: Method,
    uri: Uri,
    version: Version,
    headers: HeaderMap,
    extensions: Extensions,
}

impl RequestBuilder {
    /// Sets the HTTP method for this request.
    ///
    /// By default this is [`Method::GET`].
    #[must_use]
    pub fn method(self, method: Method) -> RequestBuilder {
        Self { method, ..self }
    }

    /// Sets the URI for this request.
    ///
    /// By default this is `/`.
    #[must_use]
    pub fn uri(self, uri: Uri) -> RequestBuilder {
        Self { uri, ..self }
    }

    /// Sets the URI string for this request.
    ///
    /// By default this is `/`.
    ///
    /// # Panics
    ///
    /// Panic when uri is invalid.
    #[must_use]
    pub fn uri_str(self, uri: impl AsRef<str>) -> RequestBuilder {
        Self {
            uri: Uri::from_str(uri.as_ref()).expect("valid url"),
            ..self
        }
    }

    /// Sets the HTTP version for this request.
    #[must_use]
    pub fn version(self, version: Version) -> RequestBuilder {
        Self { version, ..self }
    }

    /// Appends a header to this request.
    #[must_use]
    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        K: TryInto<HeaderName>,
        V: TryInto<HeaderValue>,
    {
        let key = key.try_into();
        let value = value.try_into();
        if let (Ok(key), Ok(value)) = (key, value) {
            self.headers.append(key, value);
        }
        self
    }

    /// Inserts a typed header to this request.
    #[must_use]
    pub fn typed_header<T: Header>(mut self, header: T) -> Self {
        self.headers.typed_insert(header);
        self
    }

    /// Sets the `Content-Type` header to this request.
    #[must_use]
    pub fn content_type(mut self, content_type: &str) -> Self {
        if let Ok(value) = content_type.try_into() {
            self.headers.insert(header::CONTENT_TYPE, value);
        }
        self
    }

    /// Adds an extension to this request.
    #[must_use]
    pub fn extension<T>(mut self, extension: T) -> Self
    where
        T: Clone + Send + Sync + 'static,
    {
        self.extensions.insert(extension);
        self
    }

    /// Consumes this builder, using the provided body to return a constructed
    /// [Request].
    pub fn body(self, body: impl Into<Body>) -> Request {
        Request {
            method: self.method,
            uri: self.uri,
            version: self.version,
            headers: self.headers,
            extensions: self.extensions,
            body: body.into(),
            state: Default::default(),
        }
    }

    /// Consumes this builder, using an empty body to return a constructed
    /// [Request].
    pub fn finish(self) -> Request {
        self.body(Body::empty())
    }
}
