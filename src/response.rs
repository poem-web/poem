use std::any::Any;
use std::convert::TryInto;

use crate::{
    Body, Error, Extensions, HeaderMap, HeaderName, HeaderValue, Result, StatusCode, Version,
};

struct Parts {
    status: StatusCode,
    version: Version,
    headers: HeaderMap,
    extensions: Extensions,
}

pub struct Response {
    status: StatusCode,
    version: Version,
    headers: HeaderMap,
    extensions: Extensions,
    body: Body,
}

impl Response {
    pub(crate) fn into_http_response(self) -> hyper::Response<hyper::Body> {
        let mut resp = hyper::Response::new(self.body.0);
        *resp.status_mut() = self.status.0;
        *resp.version_mut() = self.version.0;
        *resp.headers_mut() = self.headers.0;
        *resp.extensions_mut() = self.extensions;
        resp
    }

    pub fn builder() -> ResponseBuilder {
        ResponseBuilder(Ok(Parts {
            status: StatusCode::OK,
            version: Default::default(),
            headers: Default::default(),
            extensions: Default::default(),
        }))
    }

    #[inline]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    #[inline]
    pub fn set_status(&mut self, status: StatusCode) {
        self.status = status;
    }

    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    #[inline]
    pub fn version(&self) -> Version {
        self.version
    }

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

    #[inline]
    pub fn take_body(mut self) -> Body {
        std::mem::take(&mut self.body)
    }
}

pub struct ResponseBuilder(Result<Parts>);

impl ResponseBuilder {
    /// Sets the HTTP status for this response.
    ///
    /// By default this is [StatusCode::OK].
    pub fn status(self, status: StatusCode) -> Self {
        Self(self.0.map(|parts| Parts { status, ..parts }))
    }

    /// Sets the HTTP version for this response.
    ///
    /// By default this is [Version::HTTP_11]
    pub fn version(self, version: Version) -> Self {
        Self(self.0.map(|parts| Parts { version, ..parts }))
    }

    /// Appends a header to this response builder.
    ///
    /// This function will append the provided key/value as a header to the
    /// internal [HeaderMap] being constructed.
    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        K: TryInto<HeaderName>,
        K::Error: Into<Error>,
        V: TryInto<HeaderValue>,
        V::Error: Into<Error>,
    {
        Self(self.0.and_then(move |mut parts| {
            let key = key.try_into().map_err(Into::into)?;
            let value = value.try_into().map_err(Into::into)?;
            parts.headers.append(key, value);
            Ok(parts)
        }))
    }

    /// Sets the `Content-Type` header on the response.
    pub fn content_type(self, content_type: &str) -> Self {
        Self(self.0.and_then(move |mut parts| {
            let value = content_type.parse()?;
            parts.headers.append(HeaderName::CONTENT_TYPE, value);
            Ok(parts)
        }))
    }

    /// Adds an extension to this response.
    pub fn extension<T>(self, extension: T) -> Self
    where
        T: Any + Send + Sync + 'static,
    {
        Self(self.0.map(move |mut parts| {
            parts.extensions.insert(extension);
            parts
        }))
    }

    /// Consumes this builder, using the provided body to return a constructed [Response].
    ///
    /// # Errors
    ///
    /// This function may return an error if any previously configured argument
    /// failed to parse or get converted to the internal representation. For
    /// example if an invalid `head` was specified via `header("Foo",
    /// "Bar\r\n")` the error will be returned when this function is called
    /// rather than when `header` was called.
    pub fn body(self, body: Body) -> Result<Response> {
        self.0.map(move |parts| Response {
            status: parts.status,
            version: parts.version,
            headers: parts.headers,
            extensions: parts.extensions,
            body,
        })
    }
}
