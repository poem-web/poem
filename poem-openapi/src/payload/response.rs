use poem::{
    http::{header::HeaderName, HeaderMap, HeaderValue, StatusCode},
    Error, IntoResponse,
};

use crate::{
    registry::{MetaResponses, Registry},
    ApiResponse,
};

/// A response type wrapper.
///
/// Use it to modify the status code and HTTP headers.
pub struct Response<T> {
    inner: T,
    status: Option<StatusCode>,
    headers: HeaderMap,
}

impl<T> Response<T> {
    /// Create a response object.
    #[must_use]
    pub fn new(resp: T) -> Self {
        Self {
            inner: resp,
            status: None,
            headers: HeaderMap::new(),
        }
    }

    /// Sets the HTTP status for this response.
    #[must_use]
    pub fn status(self, status: StatusCode) -> Self {
        Self {
            status: Some(status),
            ..self
        }
    }

    /// Appends a header to this response.
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
}

impl<T: IntoResponse> IntoResponse for Response<T> {
    fn into_response(self) -> poem::Response {
        let mut resp = self.inner.into_response();
        if let Some(status) = self.status {
            resp.set_status(status);
        }
        resp.headers_mut().extend(self.headers);
        resp
    }
}

impl<T: ApiResponse> ApiResponse for Response<T> {
    const BAD_REQUEST_HANDLER: bool = T::BAD_REQUEST_HANDLER;

    fn meta() -> MetaResponses {
        T::meta()
    }

    fn register(registry: &mut Registry) {
        T::register(registry);
    }

    fn from_parse_request_error(err: Error) -> Self {
        Self::new(T::from_parse_request_error(err))
    }
}
