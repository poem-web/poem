use std::any::Any;
use std::convert::TryInto;

use crate::uri::Uri;
use crate::{Body, Error, Extensions, HeaderMap, HeaderName, HeaderValue, Method, Result, Version};

struct Parts {
    method: Method,
    uri: Uri,
    version: Version,
    headers: HeaderMap,
    extensions: Extensions,
}

pub struct Request {
    method: Method,
    uri: Uri,
    version: Version,
    headers: HeaderMap,
    extensions: Extensions,
    body: Body,
}

impl Request {
    pub(crate) fn from_http_request(req: hyper::Request<hyper::Body>) -> Result<Self> {
        let (parts, body) = req.into_parts();
        Ok(Self {
            method: Method::from_http_method(parts.method)?,
            uri: Uri(parts.uri),
            version: Version(parts.version),
            headers: HeaderMap(parts.headers),
            extensions: parts.extensions,
            body: Body(body),
        })
    }

    pub fn builder() -> RequestBuilder {
        RequestBuilder(Ok(Parts {
            method: Method::Get,
            uri: Default::default(),
            version: Default::default(),
            headers: Default::default(),
            extensions: Default::default(),
        }))
    }

    #[inline]
    pub fn method(&self) -> Method {
        self.method
    }

    #[inline]
    pub fn set_method(&mut self, method: Method) {
        self.method = method;
    }

    #[inline]
    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    #[inline]
    pub fn set_uri(&mut self, uri: Uri) {
        self.uri = uri;
    }

    #[inline]
    pub fn version(&self) -> Version {
        self.version
    }

    #[inline]
    pub fn set_version(&mut self, version: Version) {
        self.version = version;
    }

    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
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

    #[inline]
    pub fn take_body(&mut self) -> Body {
        std::mem::take(&mut self.body)
    }

    pub(crate) fn take_http_request(&mut self) -> hyper::Request<hyper::Body> {
        let mut http_req = http::request::Request::default();

        *http_req.method_mut() = self.method.into_http_method();
        *http_req.uri_mut() = self.uri.0.clone();
        *http_req.version_mut() = self.version.0;
        *http_req.headers_mut() = self.headers.0.clone();
        *http_req.body_mut() = self.take_body().0;

        http_req
    }
}

pub struct RequestBuilder(Result<Parts>);

impl RequestBuilder {
    /// Sets the HTTP method for this request.
    ///
    /// By default this is [Method::Get].
    pub fn method(self, method: Method) -> RequestBuilder {
        Self(self.0.map(move |parts| Parts { method, ..parts }))
    }

    /// Sets the URI for this request.
    ///
    /// By default this is `/`.
    pub fn uri<T>(self, uri: T) -> RequestBuilder
    where
        T: TryInto<Uri, Error = Error>,
    {
        Self(self.0.and_then(move |parts| {
            Ok(Parts {
                uri: uri.try_into()?,
                ..parts
            })
        }))
    }

    /// Sets the HTTP version for this request.
    pub fn version(self, version: Version) -> RequestBuilder {
        Self(self.0.map(move |parts| Parts { version, ..parts }))
    }

    /// Appends a header to this request builder.
    ///
    /// This function will append the provided key/value as a header to the
    /// internal [HeaderMap] being constructed.
    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        K: TryInto<HeaderName, Error = Error>,
        V: TryInto<HeaderValue, Error = Error>,
    {
        Self(self.0.and_then(move |mut parts| {
            let key = key.try_into()?;
            let value = value.try_into()?;
            parts.headers.append(key, value);
            Ok(parts)
        }))
    }

    /// Sets the `Content-Type` header on the request.
    pub fn content_type(self, content_type: &str) -> Self {
        Self(self.0.and_then(move |mut parts| {
            let value = content_type.parse()?;
            parts.headers.append(HeaderName::CONTENT_TYPE, value);
            Ok(parts)
        }))
    }

    /// Adds an extension to this request.
    pub fn extension<T>(self, extension: T) -> Self
    where
        T: Any + Send + Sync + 'static,
    {
        Self(self.0.map(move |mut parts| {
            parts.extensions.insert(extension);
            parts
        }))
    }

    /// Consumes this builder, using the provided body to return a constructed [Request].
    ///
    /// # Errors
    ///
    /// This function may return an error if any previously configured argument
    /// failed to parse or get converted to the internal representation. For
    /// example if an invalid `head` was specified via `header("Foo",
    /// "Bar\r\n")` the error will be returned when this function is called
    /// rather than when `header` was called.
    pub fn body(self, body: Body) -> Result<Request> {
        self.0.map(move |parts| Request {
            method: parts.method,
            uri: parts.uri,
            version: parts.version,
            headers: parts.headers,
            extensions: parts.extensions,
            body,
        })
    }
}
