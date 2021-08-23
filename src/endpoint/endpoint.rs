use std::{future::Future, sync::Arc};

use super::{After, Before, GuardEndpoint, Or};
use crate::{Guard, IntoResponse, Middleware, Request, Response};

/// An HTTP request handler.
#[async_trait::async_trait]
pub trait Endpoint: Send + Sync + 'static {
    /// Check if request matches predicate for route selection.
    #[allow(unused_variables)]
    fn check(&self, req: &Request) -> bool {
        true
    }

    /// Get the response to the request.
    async fn call(&self, req: Request) -> Response;
}

#[async_trait::async_trait]
impl<T: Endpoint + ?Sized> Endpoint for Box<T> {
    async fn call(&self, req: Request) -> Response {
        self.as_ref().call(req).await
    }
}

#[async_trait::async_trait]
impl<T: Endpoint + ?Sized> Endpoint for Arc<T> {
    async fn call(&self, req: Request) -> Response {
        self.as_ref().call(req).await
    }
}

/// Extension trait for [`Endpoint`].
pub trait EndpointExt: Endpoint {
    /// Use middleware to transform this endpoint.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{handler, middleware::AddData, route, web::Data, EndpointExt};
    ///
    /// #[handler]
    /// async fn index(Data(data): Data<&i32>) -> String {
    ///     format!("{}", data)
    /// }
    ///
    /// let app = route().at("/", index).with(AddData::new(100i32));
    /// ```
    fn with<T>(self, middleware: T) -> T::Output
    where
        T: Middleware<Self>,
        Self: Sized,
    {
        middleware.transform(self)
    }

    /// Composes a new endpoint of either this or the other endpoint.
    fn or<T>(self, other: T) -> Or<Self, T>
    where
        T: Endpoint,
        Self: Sized,
    {
        Or::new(self, other)
    }

    /// Maps the request of this endpoint.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{handler, http::StatusCode, Endpoint, EndpointExt, Error, Request, Result};
    ///
    /// #[handler]
    /// async fn index(data: String) -> String {
    ///     data
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let mut resp = index
    ///     .before(|mut req| async move {
    ///         req.set_body("abc");
    ///         req
    ///     })
    ///     .call(Request::default())
    ///     .await;
    /// assert_eq!(resp.take_body().into_string().await.unwrap(), "abc");
    /// # });
    fn before<F, Fut>(self, f: F) -> Before<Self, F>
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Request> + Send + 'static,
        Self: Sized,
    {
        Before::new(self, f)
    }

    /// Maps the response of this endpoint.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{handler, http::StatusCode, Endpoint, EndpointExt, Error, Request, Result};
    ///
    /// #[handler]
    /// async fn index() -> &'static str {
    ///     "abc"
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let mut resp = index
    ///     .after(|mut resp| async move {
    ///         resp.take_body().into_string().await.unwrap() + "def"
    ///     })
    ///     .call(Request::default())
    ///     .await;
    /// assert_eq!(resp.take_body().into_string().await.unwrap(), "abcdef");
    /// # });
    fn after<F, Fut, R>(self, f: F) -> After<Self, F>
    where
        F: Fn(Response) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse,
        Self: Sized,
    {
        After::new(self, f)
    }

    /// Add guard to the endpoint.
    fn guard<T>(self, guard: T) -> GuardEndpoint<Self, T>
    where
        T: Guard,
        Self: Sized,
    {
        GuardEndpoint::new(self, guard)
    }
}

impl<T: Endpoint> EndpointExt for T {}

#[cfg(test)]
mod test {
    use crate::{
        http::{Method, StatusCode},
        *,
    };

    #[handler(internal)]
    async fn handler_request(method: Method) -> String {
        method.to_string()
    }

    #[handler(internal)]
    async fn handler() -> &'static str {
        "abc"
    }

    #[handler(internal)]
    async fn handler_err() -> Result<&'static str> {
        Err(Error::status(StatusCode::BAD_REQUEST))
    }

    #[tokio::test]
    async fn test_before() {
        assert_eq!(
            handler_request
                .before(|mut req| async move {
                    req.set_method(Method::POST);
                    req
                })
                .call(Request::default())
                .await
                .take_body()
                .into_string()
                .await
                .unwrap(),
            "POST"
        );
    }

    #[tokio::test]
    async fn test_after() {
        assert_eq!(
            handler
                .after(|_| async { "def" })
                .call(Request::default())
                .await
                .take_body()
                .into_string()
                .await
                .unwrap(),
            "def"
        );

        let resp = handler
            .after(|_| async { Err::<(), _>(Error::status(StatusCode::FORBIDDEN)) })
            .call(Request::default())
            .await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}
