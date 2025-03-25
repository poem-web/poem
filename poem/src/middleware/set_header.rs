use crate::{
    Endpoint, IntoResponse, Middleware, Request, Response, Result,
    http::{HeaderValue, header::HeaderName},
};

#[derive(Debug, Clone)]
enum Action {
    Override(HeaderName, HeaderValue),
    Append(HeaderName, HeaderValue),
}

/// Middleware to override or append headers to a response.
///
/// # Example
///
/// ```
/// use poem::{
///     Endpoint, EndpointExt, Request, Route, get, handler,
///     http::{HeaderValue, StatusCode},
///     middleware::SetHeader,
///     test::TestClient,
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
/// let resp = TestClient::new(app).get("/").send().await;
/// resp.assert_status_is_ok();
/// resp.assert_header_all("MyHeader1", ["a", "b"]);
/// resp.assert_header_all("MyHeader2", ["b"]);
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

    /// Inserts a header into the response.
    ///
    /// If a previous value exists for the same header, it will
    /// be overridden.
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

    /// Appends a header to the response.
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

/// Endpoint for the SetHeader middleware.
pub struct SetHeaderEndpoint<E> {
    inner: E,
    actions: Vec<Action>,
}

impl<E: Endpoint> Endpoint for SetHeaderEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let mut resp = self.inner.call(req).await?.into_response();
        let headers = resp.headers_mut();

        for action in &self.actions {
            match action {
                Action::Override(name, value) => {
                    headers.insert(name, value.clone());
                }
                Action::Append(name, value) => {
                    headers.append(name, value.clone());
                }
            }
        }

        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EndpointExt, handler, test::TestClient};

    #[tokio::test]
    async fn test_set_header() {
        #[handler(internal)]
        fn index() {}

        let cli = TestClient::new(
            index.with(
                SetHeader::new()
                    .overriding("custom-a", "a")
                    .overriding("custom-a", "b")
                    .appending("custom-b", "a")
                    .appending("custom-b", "b"),
            ),
        );

        let resp = cli.get("/").send().await;

        resp.assert_status_is_ok();
        resp.assert_header_all("custom-a", ["b"]);
        resp.assert_header_all("custom-b", ["a", "b"]);
    }
}
