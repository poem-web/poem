use std::convert::TryInto;

use crate::{
    http::{header::HeaderName, HeaderValue},
    Endpoint, IntoResponse, Middleware, Request, Response, Result,
};

#[derive(Debug, Clone)]
enum Action {
    Override(HeaderName, HeaderValue),
    Append(HeaderName, HeaderValue),
}

/// Middleware for override/append headers to response.
///
/// # Example
///
/// ```
/// use poem::{
///     get, handler,
///     http::{HeaderValue, StatusCode},
///     middleware::SetHeader,
///     Endpoint, EndpointExt, Request, Route,
/// };
///
/// #[handler]
/// fn index() -> &'static str {
///     "hello"
/// }
///
/// let app = Route::new().at("/", get(index)).with(
///     SetHeader::new()
///         .appending("MyHeader1", "a")
///         .appending("MyHeader1", "b")
///         .overriding("MyHeader2", "a")
///         .overriding("MyHeader2", "b"),
/// );
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = app.call(Request::default()).await.unwrap();
/// assert_eq!(resp.status(), StatusCode::OK);
/// assert_eq!(
///     resp.headers()
///         .get_all("MyHeader1")
///         .iter()
///         .collect::<Vec<_>>(),
///     vec![HeaderValue::from_static("a"), HeaderValue::from_static("b")]
/// );
/// assert_eq!(
///     resp.headers()
///         .get_all("MyHeader2")
///         .iter()
///         .collect::<Vec<_>>(),
///     vec![HeaderValue::from_static("b")]
/// );
/// # });
/// ```
#[derive(Default)]
pub struct SetHeader {
    actions: Vec<Action>,
}

impl SetHeader {
    /// Create new `SetHeader` middleware.
    #[must_use]
    pub fn new() -> Self {
        Default::default()
    }

    /// Inserts a header to response.
    ///
    /// If a previous value exists for the same header, it is
    /// removed and replaced with the new header value.
    #[must_use]
    pub fn overriding<K, V>(mut self, key: K, value: V) -> Self
    where
        K: TryInto<HeaderName>,
        V: TryInto<HeaderValue>,
    {
        let key = key.try_into();
        let value = value.try_into();
        if let (Ok(key), Ok(value)) = (key, value) {
            self.actions.push(Action::Override(key, value));
        }
        self
    }

    /// Appends a header to response.
    ///
    /// If previous values exist, the header will have multiple values.
    #[must_use]
    pub fn appending<K, V>(mut self, key: K, value: V) -> Self
    where
        K: TryInto<HeaderName>,
        V: TryInto<HeaderValue>,
    {
        let key = key.try_into();
        let value = value.try_into();
        if let (Ok(key), Ok(value)) = (key, value) {
            self.actions.push(Action::Append(key, value));
        }
        self
    }
}

impl<E: Endpoint> Middleware<E> for SetHeader {
    type Output = SetHeaderEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        SetHeaderEndpoint {
            inner: ep,
            actions: self.actions.clone(),
        }
    }
}

/// Endpoint for SetHeader middleware.
pub struct SetHeaderEndpoint<E> {
    inner: E,
    actions: Vec<Action>,
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for SetHeaderEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let mut resp = self.inner.call(req).await?.into_response();
        let headers = resp.headers_mut();

        for action in &self.actions {
            match action {
                Action::Override(name, value) => {
                    headers.insert(name.clone(), value.clone());
                }
                Action::Append(name, value) => {
                    headers.append(name.clone(), value.clone());
                }
            }
        }

        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{handler, EndpointExt};

    #[tokio::test]
    async fn test_set_header() {
        #[handler(internal)]
        fn index() {}

        let resp = index
            .with(
                SetHeader::new()
                    .overriding("custom-a", "a")
                    .overriding("custom-a", "b")
                    .appending("custom-b", "a")
                    .appending("custom-b", "b"),
            )
            .call(Request::default())
            .await
            .unwrap();

        assert_eq!(
            resp.headers()
                .get_all("custom-a")
                .into_iter()
                .filter_map(|value| value.to_str().ok())
                .collect::<Vec<_>>(),
            vec!["b"]
        );

        assert_eq!(
            resp.headers()
                .get_all("custom-b")
                .into_iter()
                .filter_map(|value| value.to_str().ok())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }
}
