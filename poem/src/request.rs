use std::{
    any::Any,
    convert::TryInto,
    fmt::{self, Debug, Formatter},
};

#[cfg(feature = "websocket")]
use hyper::upgrade::OnUpgrade;
use once_cell::sync::Lazy;
#[cfg(feature = "websocket")]
use parking_lot::Mutex;

use crate::{
    body::Body,
    http::{
        header::{self, HeaderMap, HeaderName, HeaderValue},
        Extensions, Method, Uri, Version,
    },
    route::PathParams,
    web::{CookieJar, RemoteAddr},
    RequestBody,
};

pub(crate) struct RequestState {
    pub(crate) remote_addr: RemoteAddr,
    pub(crate) original_uri: Uri,
    pub(crate) match_params: PathParams,
    pub(crate) cookie_jar: Option<CookieJar>,
    #[cfg(feature = "websocket")]
    pub(crate) on_upgrade: Mutex<Option<OnUpgrade>>,
}

impl Default for RequestState {
    fn default() -> Self {
        static UNKNOWN_REMOTE_ADDR: Lazy<RemoteAddr> = Lazy::new(|| RemoteAddr::new("unknown"));

        Self {
            remote_addr: UNKNOWN_REMOTE_ADDR.clone(),
            original_uri: Default::default(),
            match_params: Default::default(),
            cookie_jar: Default::default(),
            #[cfg(feature = "websocket")]
            on_upgrade: Default::default(),
        }
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

impl From<(http::Request<hyper::Body>, RemoteAddr)> for Request {
    fn from((req, remote_addr): (http::Request<hyper::Body>, RemoteAddr)) -> Self {
        #[allow(unused_mut)]
        let (mut parts, body) = req.into_parts();
        #[cfg(feature = "websocket")]
        let on_upgrade = Mutex::new(parts.extensions.remove::<OnUpgrade>());

        Self {
            method: parts.method,
            uri: parts.uri.clone(),
            version: parts.version,
            headers: parts.headers,
            extensions: parts.extensions,
            body: Body(body),
            state: RequestState {
                remote_addr,
                original_uri: parts.uri,
                match_params: Default::default(),
                cookie_jar: None,
                #[cfg(feature = "websocket")]
                on_upgrade,
            },
        }
    }
}

impl From<Request> for hyper::Request<hyper::Body> {
    fn from(req: Request) -> Self {
        let mut hyper_req = http::Request::builder()
            .method(req.method)
            .uri(req.uri)
            .version(req.version)
            .body(req.body.into())
            .unwrap();
        *hyper_req.headers_mut() = req.headers;
        *hyper_req.extensions_mut() = req.extensions;
        hyper_req
    }
}

impl Request {
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

    /// Returns the path parameter with the specified `name`.
    pub fn path_param(&self, name: &str) -> Option<&str> {
        self.state
            .match_params
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
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

    /// Returns a reference to the [`CookieJar`]
    #[inline]
    pub fn cookie(&self) -> &CookieJar {
        match &self.state.cookie_jar {
            Some(cookie_jar) => cookie_jar,
            None => panic!(
                "To use the `CookieJar` extractor, the `CookieJarManager` middleware is required."
            ),
        }
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

    /// Sets the HTTP version for this request.
    #[must_use]
    pub fn version(self, version: Version) -> RequestBuilder {
        Self { version, ..self }
    }

    /// Appends a header to this response builder.
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

    /// Sets the `Content-Type` header on the response.
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
        T: Any + Send + Sync + 'static,
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
