use std::{any::Any, convert::TryFrom};

use crate::{
    error::ErrorBodyHasBeenTaken,
    http::{
        header::{self, HeaderMap, HeaderName, HeaderValue},
        Extensions, StatusCode, Version,
    },
    Body, Error, Result,
};

struct Parts {
    status: StatusCode,
    version: Version,
    headers: HeaderMap,
    extensions: Extensions,
}

/// Represents an HTTP response.
pub struct Response {
    status: StatusCode,
    version: Version,
    headers: HeaderMap,
    extensions: Extensions,
    body: Option<Body>,
}

impl Response {
    pub(crate) fn into_http_response(self) -> hyper::Response<hyper::Body> {
        let mut resp = hyper::Response::new(
            self.body
                .map(|body| body.0)
                .unwrap_or_else(|| hyper::Body::empty()),
        );
        *resp.status_mut() = self.status;
        *resp.version_mut() = self.version;
        *resp.headers_mut() = self.headers;
        *resp.extensions_mut() = self.extensions;
        resp
    }

    /// Creates a response builder.
    pub fn builder() -> ResponseBuilder {
        ResponseBuilder(Ok(Parts {
            status: StatusCode::OK,
            version: Default::default(),
            headers: Default::default(),
            extensions: Default::default(),
        }))
    }

    /// Returns the associated status code.
    #[inline]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Sets the status code for this response.
    #[inline]
    pub fn set_status(&mut self, status: StatusCode) {
        self.status = status;
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
    pub fn set_body(&mut self, body: Body) {
        self.body = Some(body);
    }

    /// Take the body from this response and sets the body to empty.
    #[inline]
    #[must_use]
    pub fn take_body(&mut self) -> Result<Body> {
        self.body.take().ok_or(ErrorBodyHasBeenTaken.into())
    }
}

/// An response builder.
pub struct ResponseBuilder(Result<Parts>);

impl ResponseBuilder {
    /// Sets the HTTP status for this response.
    ///
    /// By default this is [`StatusCode::OK`].
    #[must_use]
    pub fn status(self, status: StatusCode) -> Self {
        Self(self.0.map(|parts| Parts { status, ..parts }))
    }

    /// Sets the HTTP version for this response.
    ///
    /// By default this is [`Version::HTTP_11`]
    #[must_use]
    pub fn version(self, version: Version) -> Self {
        Self(self.0.map(|parts| Parts { version, ..parts }))
    }

    /// Appends a header to this response builder.
    ///
    /// This function will append the provided key/value as a header to the
    /// internal [`HeaderMap`] being constructed.
    #[must_use]
    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<Error>,
    {
        Self(self.0.and_then(move |mut parts| {
            let key = <HeaderName as TryFrom<K>>::try_from(key).map_err(Into::into)?;
            let value = <HeaderValue as TryFrom<V>>::try_from(value).map_err(Into::into)?;
            parts.headers.append(key, value);
            Ok(parts)
        }))
    }

    /// Sets the `Content-Type` header on the response.
    #[must_use]
    pub fn content_type(self, content_type: &str) -> Self {
        Self(self.0.and_then(move |mut parts| {
            let value = content_type.parse()?;
            parts.headers.append(header::CONTENT_TYPE, value);
            Ok(parts)
        }))
    }

    /// Adds an extension to this response.
    #[must_use]
    pub fn extension<T>(self, extension: T) -> Self
    where
        T: Any + Send + Sync + 'static,
    {
        Self(self.0.map(move |mut parts| {
            parts.extensions.insert(extension);
            parts
        }))
    }

    /// Consumes this builder, using the provided body to return a constructed
    /// [Response].
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
            body: Some(body),
        })
    }
}
