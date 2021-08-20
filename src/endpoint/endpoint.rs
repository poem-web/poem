use std::{future::Future, sync::Arc};

use super::{AndThen, GuardEndpoint, Map, MapErr, MapOk, MapRequest, MapToResponse, Or};
use crate::{Error, Guard, IntoResponse, Middleware, Request, Response, Result};

/// An HTTP request handler.
#[async_trait::async_trait]
pub trait Endpoint: Send + Sync + 'static {
    /// Check if request matches predicate for route selection.
    #[allow(unused_variables)]
    fn check(&self, req: &Request) -> bool {
        true
    }

    /// Get the response to the request.
    async fn call(&self, req: Request) -> Result<Response>;
}

#[async_trait::async_trait]
impl<T: Endpoint + ?Sized> Endpoint for Box<T> {
    async fn call(&self, req: Request) -> Result<Response> {
        self.as_ref().call(req).await
    }
}

#[async_trait::async_trait]
impl<T: Endpoint + ?Sized> Endpoint for Arc<T> {
    async fn call(&self, req: Request) -> Result<Response> {
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
    /// use poem::{get, middleware::AddData, route, web::Data, EndpointExt};
    ///
    /// #[get]
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
    /// let res = index
    ///     .map_request(|mut req| async move {
    ///         req.set_body("abc");
    ///         Ok(req)
    ///     })
    ///     .call(Request::default())
    ///     .await;
    /// match res {
    ///     Ok(mut resp) => resp.take_body().into_string().await.unwrap() == "abc",
    ///     Err(_) => unreachable!(),
    /// }
    /// # });
    fn map_request<F, Fut, Err>(self, f: F) -> MapRequest<Self, F>
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Request, Err>> + Send + 'static,
        Err: Into<Error>,
        Self: Sized,
    {
        MapRequest::new(self, f)
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
    /// let res = index
    ///     .map(|mut res| async move {
    ///         match res {
    ///             Ok(mut resp) => Ok(resp.take_body().into_string().await.unwrap() + "def"),
    ///             Err(err) => Err(err),
    ///         }
    ///     })
    ///     .call(Request::default())
    ///     .await;
    /// match res {
    ///     Ok(mut resp) => resp.take_body().into_string().await.unwrap() == "abcdef",
    ///     Err(_) => unreachable!(),
    /// }
    /// # });
    fn map<F, Fut, R>(self, f: F) -> Map<Self, F>
    where
        F: Fn(Result<Response>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<R>> + Send + 'static,
        R: IntoResponse,
        Self: Sized,
    {
        Map::new(self, f)
    }

    /// Calls `f` if the result is `Ok`, otherwise returns the `Err` value of
    /// self.
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
    /// let res = index
    ///     .and_then(
    ///         |mut resp| async move { Ok(resp.take_body().into_string().await.unwrap() + "def") },
    ///     )
    ///     .call(Request::default())
    ///     .await;
    /// match res {
    ///     Ok(mut resp) => resp.take_body().into_string().await.unwrap() == "abcdef",
    ///     Err(_) => unreachable!(),
    /// }
    /// # });
    /// ```
    fn and_then<F, Fut, R>(self, f: F) -> AndThen<Self, F>
    where
        F: Fn(Response) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<R>> + Send + 'static,
        R: IntoResponse,
        Self: Sized,
    {
        AndThen::new(self, f)
    }

    /// Maps the response of this endpoint.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{handler, http::StatusCode, Endpoint, EndpointExt, Error, Request};
    ///
    /// #[handler]
    /// async fn index() -> &'static str {
    ///     "abc"
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let res = index
    ///     .map_ok(|mut resp| async move { resp.take_body().into_string().await.unwrap() + "def" })
    ///     .call(Request::default())
    ///     .await;
    /// match res {
    ///     Ok(mut resp) => resp.take_body().into_string().await.unwrap() == "abcdef",
    ///     Err(_) => unreachable!(),
    /// }
    /// # });
    /// ```
    fn map_ok<F, Fut, R>(self, f: F) -> MapOk<Self, F>
    where
        F: Fn(Response) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse,
        Self: Sized,
    {
        MapOk::new(self, f)
    }

    /// Maps the error of this endpoint.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{handler, http::StatusCode, Endpoint, EndpointExt, Error, Request};
    ///
    /// #[handler]
    /// async fn index() -> Result<(), Error> {
    ///     Err(Error::status(StatusCode::BAD_GATEWAY))
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let res = index
    ///     .map_err(|_| async { Error::status(StatusCode::FORBIDDEN) })
    ///     .call(Request::default())
    ///     .await;
    /// match res {
    ///     Ok(_) => unreachable!(),
    ///     Err(err) => err.as_response().status() == StatusCode::FORBIDDEN,
    /// }
    /// # });
    /// ```
    fn map_err<F, Fut>(self, f: F) -> MapErr<Self, F>
    where
        F: Fn(Error) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Error> + Send + 'static,
        Self: Sized,
    {
        MapErr::new(self, f)
    }

    /// Wrap this endpoint that does not return an error.
    ///
    /// if this endpoint returns an error, the error will be converted into a
    /// response using
    /// [`ResponseError::as_response`](crate::ResponseError::as_response).
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{handler, http::StatusCode, Endpoint, EndpointExt, Error, Request};
    ///
    /// #[handler]
    /// async fn index() -> Result<(), Error> {
    ///     Err(Error::status(StatusCode::BAD_GATEWAY))
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let res = index.map_to_response().call(Request::default()).await;
    /// match res {
    ///     Ok(resp) => assert_eq!(resp.status(), StatusCode::BAD_GATEWAY),
    ///     Err(_) => unreachable!(),
    /// }
    /// # });
    /// ```
    fn map_to_response(self) -> MapToResponse<Self>
    where
        Self: Sized,
    {
        MapToResponse::new(self)
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
    async fn test_map_request() {
        assert_eq!(
            handler_request
                .map_request(|mut req| async move {
                    req.set_method(Method::POST);
                    Ok(req)
                })
                .call(Request::default())
                .await
                .unwrap()
                .take_body()
                .into_string()
                .await
                .unwrap(),
            "POST"
        );
    }

    #[tokio::test]
    async fn test_map() {
        assert_eq!(
            handler
                .map(|_| async { Ok("def") })
                .call(Request::default())
                .await
                .unwrap()
                .take_body()
                .into_string()
                .await
                .unwrap(),
            "def"
        );

        let err = handler
            .map(|_| async { Err::<(), _>(Error::status(StatusCode::FORBIDDEN)) })
            .call(Request::default())
            .await
            .unwrap_err();
        assert_eq!(err.as_response().status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_and_then() {
        assert_eq!(
            handler
                .and_then(|mut resp| async move {
                    Ok(resp.take_body().into_string().await.unwrap() + "def")
                })
                .call(Request::default())
                .await
                .unwrap()
                .take_body()
                .into_string()
                .await
                .unwrap(),
            "abcdef"
        );

        let err = handler_err
            .and_then(|mut resp| async move {
                Ok(resp.take_body().into_string().await.unwrap() + "def")
            })
            .call(Request::default())
            .await
            .unwrap_err();
        assert_eq!(err.as_response().status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_map_ok() {
        assert_eq!(
            handler
                .map_ok(
                    |mut resp| async move { resp.take_body().into_string().await.unwrap() + "def" }
                )
                .call(Request::default())
                .await
                .unwrap()
                .take_body()
                .into_string()
                .await
                .unwrap(),
            "abcdef"
        );

        let err = handler_err
            .map_ok(|mut resp| async move { resp.take_body().into_string().await.unwrap() + "def" })
            .call(Request::default())
            .await
            .unwrap_err();
        assert_eq!(err.as_response().status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_map_err() {
        assert_eq!(
            handler
                .map_err(|_| async move { Error::status(StatusCode::BAD_GATEWAY) })
                .call(Request::default())
                .await
                .unwrap()
                .take_body()
                .into_string()
                .await
                .unwrap(),
            "abc"
        );

        let err = handler_err
            .map_err(|_| async move { Error::status(StatusCode::BAD_GATEWAY) })
            .call(Request::default())
            .await
            .unwrap_err();
        assert_eq!(err.as_response().status(), StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn test_map_to_response() {
        assert_eq!(
            handler_err
                .map_to_response()
                .call(Request::default())
                .await
                .unwrap()
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
}
