use std::{future::Future, sync::Arc};

use super::{AndThen, Map, MapErr, MapOk, MapRequest};
use crate::{Error, Middleware, Request, Response, Result};

/// An HTTP request handler.
#[async_trait::async_trait]
pub trait Endpoint: Send + Sync + 'static {
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
    /// use poem::{get, handler, middleware::AddData, route, web::Data, EndpointExt};
    ///
    /// #[handler]
    /// async fn index(Data(data): Data<&i32>) -> String {
    ///     format!("{}", data)
    /// }
    ///
    /// let app = route().at("/", get(index)).with(AddData::new(100i32));
    /// ```
    fn with<T>(self, middleware: T) -> T::Output
    where
        T: Middleware<Self>,
        Self: Sized,
    {
        middleware.transform(self)
    }

    /// Maps the request of this endpoint.
    fn map_request<F, Fut>(self, f: F) -> MapRequest<Self, F>
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Request>> + Send + 'static,
        Self: Sized,
    {
        MapRequest::new(self, f)
    }

    /// Maps the response of this endpoint.
    fn map<F, Fut>(self, f: F) -> Map<Self, F>
    where
        F: Fn(Result<Response>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
        Self: Sized,
    {
        Map::new(self, f)
    }

    /// calls `f` if the result is `Ok`, otherwise returns the `Err` value of
    /// self.
    fn and_then<F, Fut>(self, f: F) -> AndThen<Self, F>
    where
        F: Fn(Response) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
        Self: Sized,
    {
        AndThen::new(self, f)
    }

    /// Maps the response of this endpoint.
    fn map_ok<F, Fut>(self, f: F) -> MapOk<Self, F>
    where
        F: Fn(Response) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
        Self: Sized,
    {
        MapOk::new(self, f)
    }

    /// Maps the error of this endpoint.
    fn map_err<F, Fut>(self, f: F) -> MapErr<Self, F>
    where
        F: Fn(Error) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Error> + Send + 'static,
        Self: Sized,
    {
        MapErr::new(self, f)
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
                .map(|_| async { Ok("def".into()) })
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
            .map(|_| async { Err(Error::status(StatusCode::FORBIDDEN)) })
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
                    Ok((resp.take_body().into_string().await.unwrap() + "def").into())
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
                Ok((resp.take_body().into_string().await.unwrap() + "def").into())
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
                .map_ok(|mut resp| async move {
                    (resp.take_body().into_string().await.unwrap() + "def").into()
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
            .map_ok(|mut resp| async move {
                (resp.take_body().into_string().await.unwrap() + "def").into()
            })
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
}
