use crate::{
    endpoint::BoxEndpoint, error::MethodNotAllowedError, http::Method, Endpoint, EndpointExt,
    IntoEndpoint, Request, Response, Result,
};

/// Routing object for HTTP methods
///
/// # Errors
///
/// - [`MethodNotAllowedError`]
///
/// # Example
///
/// ```
/// use poem::{
///     handler,
///     http::{Method, StatusCode},
///     Endpoint, Request, RouteMethod,
/// };
///
/// #[handler]
/// fn handle_get() -> &'static str {
///     "get"
/// }
///
/// #[handler]
/// fn handle_post() -> &'static str {
///     "post"
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let route_method = RouteMethod::new().get(handle_get).post(handle_post);
///
/// let resp = route_method
///     .get_response(Request::builder().method(Method::GET).finish())
///     .await;
/// assert_eq!(resp.status(), StatusCode::OK);
/// assert_eq!(resp.into_body().into_string().await.unwrap(), "get");
///
/// let resp = route_method
///     .get_response(Request::builder().method(Method::POST).finish())
///     .await;
/// assert_eq!(resp.status(), StatusCode::OK);
/// assert_eq!(resp.into_body().into_string().await.unwrap(), "post");
///
/// let resp = route_method
///     .get_response(Request::builder().method(Method::PUT).finish())
///     .await;
/// assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
/// # });
/// ```
#[derive(Default)]
pub struct RouteMethod {
    methods: Vec<(Method, BoxEndpoint<'static>)>,
}

impl RouteMethod {
    /// Create a `RouteMethod` object.
    pub fn new() -> Self {
        Default::default()
    }

    /// Sets the endpoint for the specified `method`.
    #[must_use]
    pub fn method<E>(mut self, method: Method, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.methods
            .push((method, ep.into_endpoint().map_to_response().boxed()));
        self
    }

    /// Sets the endpoint for `GET`.
    #[must_use]
    pub fn get<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.method(Method::GET, ep)
    }

    /// Sets the endpoint for `POST`.
    #[must_use]
    pub fn post<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.method(Method::POST, ep)
    }

    /// Sets the endpoint for `PUT`.
    #[must_use]
    pub fn put<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.method(Method::PUT, ep)
    }

    /// Sets the endpoint for `DELETE`.
    #[must_use]
    pub fn delete<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.method(Method::DELETE, ep)
    }

    /// Sets the endpoint for `HEAD`.
    #[must_use]
    pub fn head<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.method(Method::HEAD, ep)
    }

    /// Sets the endpoint for `OPTIONS`.
    #[must_use]
    pub fn options<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.method(Method::OPTIONS, ep)
    }

    /// Sets the endpoint for `CONNECT`.
    #[must_use]
    pub fn connect<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.method(Method::CONNECT, ep)
    }

    /// Sets the endpoint for `PATCH`.
    #[must_use]
    pub fn patch<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.method(Method::PATCH, ep)
    }

    /// Sets the endpoint for `TRACE`.
    #[must_use]
    pub fn trace<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint,
        E::Endpoint: 'static,
    {
        self.method(Method::TRACE, ep)
    }
}

#[async_trait::async_trait]
impl Endpoint for RouteMethod {
    type Output = Response;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        match self
            .methods
            .iter()
            .find(|(method, _)| method == req.method())
            .map(|(_, ep)| ep)
        {
            Some(ep) => ep.call(req).await,
            None => {
                if req.method() == Method::HEAD {
                    req.set_method(Method::GET);
                    let mut resp = self.call(req).await?;
                    resp.set_body(());
                    return Ok(resp);
                }
                Err(MethodNotAllowedError.into())
            }
        }
    }
}

/// A helper function, similar to `RouteMethod::new().get(ep)`.
pub fn get<E>(ep: E) -> RouteMethod
where
    E: IntoEndpoint,
    E::Endpoint: 'static,
{
    RouteMethod::new().get(ep)
}

/// A helper function, similar to `RouteMethod::new().post(ep)`.
pub fn post<E>(ep: E) -> RouteMethod
where
    E: IntoEndpoint,
    E::Endpoint: 'static,
{
    RouteMethod::new().post(ep)
}

/// A helper function, similar to `RouteMethod::new().put(ep)`.
pub fn put<E>(ep: E) -> RouteMethod
where
    E: IntoEndpoint,
    E::Endpoint: 'static,
{
    RouteMethod::new().put(ep)
}

/// A helper function, similar to `RouteMethod::new().delete(ep)`.
pub fn delete<E>(ep: E) -> RouteMethod
where
    E: IntoEndpoint,
    E::Endpoint: 'static,
{
    RouteMethod::new().delete(ep)
}

/// A helper function, similar to `RouteMethod::new().head(ep)`.
pub fn head<E>(ep: E) -> RouteMethod
where
    E: IntoEndpoint,
    E::Endpoint: 'static,
{
    RouteMethod::new().head(ep)
}

/// A helper function, similar to `RouteMethod::new().options(ep)`.
pub fn options<E>(ep: E) -> RouteMethod
where
    E: IntoEndpoint,
    E::Endpoint: 'static,
{
    RouteMethod::new().options(ep)
}

/// A helper function, similar to `RouteMethod::new().connect(ep)`.
pub fn connect<E>(ep: E) -> RouteMethod
where
    E: IntoEndpoint,
    E::Endpoint: 'static,
{
    RouteMethod::new().connect(ep)
}

/// A helper function, similar to `RouteMethod::new().patch(ep)`.
pub fn patch<E>(ep: E) -> RouteMethod
where
    E: IntoEndpoint,
    E::Endpoint: 'static,
{
    RouteMethod::new().patch(ep)
}

/// A helper function, similar to `RouteMethod::new().trace(ep)`.
pub fn trace<E>(ep: E) -> RouteMethod
where
    E: IntoEndpoint,
    E::Endpoint: 'static,
{
    RouteMethod::new().trace(ep)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        handler,
        http::{Method, StatusCode},
        test::TestClient,
    };

    #[tokio::test]
    async fn method_not_allowed() {
        let resp = TestClient::new(RouteMethod::new()).get("/").send().await;
        resp.assert_status(StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn route_method() {
        #[handler(internal)]
        fn index() -> &'static str {
            "hello"
        }

        for method in &[
            Method::GET,
            Method::POST,
            Method::DELETE,
            Method::PUT,
            Method::HEAD,
            Method::OPTIONS,
            Method::CONNECT,
            Method::PATCH,
            Method::TRACE,
        ] {
            let route = RouteMethod::new().method(method.clone(), index).post(index);
            let resp = TestClient::new(route)
                .request(method.clone(), "/")
                .send()
                .await;
            resp.assert_status_is_ok();
            resp.assert_text("hello").await;
        }

        macro_rules! test_method {
            ($(($id:ident, $method:ident)),*) => {
                $(
                let route = RouteMethod::new().$id(index).post(index);
                let resp = TestClient::new(route).request(Method::$method, "/").send().await;
                resp.assert_status_is_ok();
                resp.assert_text("hello").await;
                )*
            };
        }

        test_method!(
            (get, GET),
            (post, POST),
            (delete, DELETE),
            (put, PUT),
            (head, HEAD),
            (options, OPTIONS),
            (connect, CONNECT),
            (patch, PATCH),
            (trace, TRACE)
        );
    }

    #[tokio::test]
    async fn head_method() {
        #[handler(internal)]
        fn index() -> &'static str {
            "hello"
        }

        let route = RouteMethod::new().get(index);
        let resp = TestClient::new(route).head("/").send().await;
        resp.assert_status_is_ok();
        resp.assert_text("").await;
    }
}
