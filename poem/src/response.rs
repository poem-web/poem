use std::{
    any::Any,
    convert::TryInto,
    fmt::{self, Debug, Formatter},
};

use crate::{
    http::{
        header::{self, HeaderMap, HeaderName, HeaderValue},
        Extensions, StatusCode, Version,
    },
    Body, Error,
};

/// Represents an HTTP response.
#[derive(Default)]
pub struct Response {
    status: StatusCode,
    version: Version,
    headers: HeaderMap,
    extensions: Extensions,
    body: Body,
}

impl Debug for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("status", &self.status)
            .field("version", &self.version)
            .field("headers", &self.headers)
            .finish()
    }
}

impl<T: Into<Body>> From<T> for Response {
    fn from(body: T) -> Self {
        Response::builder().body(body.into())
    }
}

impl From<StatusCode> for Response {
    fn from(status: StatusCode) -> Self {
        Response::builder().status(status).finish()
    }
}

impl From<Error> for Response {
    fn from(err: Error) -> Self {
        err.as_response()
    }
}

impl<T: Into<Body>> From<(StatusCode, T)> for Response {
    fn from((status, body): (StatusCode, T)) -> Self {
        Response::builder().status(status).body(body.into())
    }
}

impl From<Response> for hyper::Response<hyper::Body> {
    fn from(resp: Response) -> Self {
        let mut hyper_resp = hyper::Response::new(resp.body.into());
        *hyper_resp.status_mut() = resp.status;
        *hyper_resp.version_mut() = resp.version;
        *hyper_resp.headers_mut() = resp.headers;
        *hyper_resp.extensions_mut() = resp.extensions;
        hyper_resp
    }
}

impl From<hyper::Response<hyper::Body>> for Response {
    fn from(hyper_resp: hyper::Response<hyper::Body>) -> Self {
        let (parts, body) = hyper_resp.into_parts();
        Response {
            status: parts.status,
            version: parts.version,
            headers: parts.headers,
            extensions: parts.extensions,
            body: body.into(),
        }
    }
}

impl Response {
    /// Creates a response builder.
    pub fn builder() -> ResponseBuilder {
        ResponseBuilder {
            status: StatusCode::OK,
            version: Default::default(),
            headers: Default::default(),
            extensions: Default::default(),
        }
    }

    /// Returns the associated status code.
    #[inline]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Check if status is within 200-299.
    #[inline]
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    /// Sets the status code for this response.
    #[inline]
    pub fn set_status(&mut self, status: StatusCode) {
        self.status = status;
    }

    /// Returns the content type of this response.
    pub fn content_type(&self) -> Option<&str> {
        self.headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
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

    /// Returns the associated version.
    #[inline]
    pub fn version(&self) -> Version {
        self.version
    }

    /// Sets the version for this response.
    #[inline]
    pub fn set_version(&mut self, version: Version) {
        self.version = version;
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

    /// Sets the body for this response.
    pub fn set_body(&mut self, body: impl Into<Body>) {
        self.body = body.into();
    }

    /// Take the body from this response and sets the body to empty.
    #[inline]
    pub fn take_body(&mut self) -> Body {
        std::mem::take(&mut self.body)
    }
}

/// An response builder.
pub struct ResponseBuilder {
    status: StatusCode,
    version: Version,
    headers: HeaderMap,
    extensions: Extensions,
}

impl ResponseBuilder {
    /// Sets the HTTP status for this response.
    ///
    /// By default this is [`StatusCode::OK`].
    #[must_use]
    pub fn status(self, status: StatusCode) -> Self {
        Self { status, ..self }
    }

    /// Sets the HTTP version for this response.
    ///
    /// By default this is [`Version::HTTP_11`]
    #[must_use]
    pub fn version(self, version: Version) -> Self {
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
            self.headers.insert(key, value);
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

    /// Adds an extension to this response.
    #[must_use]
    pub fn extension<T>(mut self, extension: T) -> Self
    where
        T: Any + Send + Sync + 'static,
    {
        self.extensions.insert(extension);
        self
    }

    /// Consumes this builder, using the provided body to return a constructed
    /// [Response].
    pub fn body(self, body: impl Into<Body>) -> Response {
        Response {
            status: self.status,
            version: self.version,
            headers: self.headers,
            extensions: self.extensions,
            body: body.into(),
        }
    }

    /// Consumes this builder, using an empty body to return a constructed
    /// [Response].
    pub fn finish(self) -> Response {
        self.body(Body::empty())
    }
}
