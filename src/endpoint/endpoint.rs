use std::{future::Future, sync::Arc};

use super::{After, AndThen, Before, MapErr, MapOk, MapToResponse, MapToResult};
use crate::{Error, IntoResponse, Middleware, Request, Result};

/// An HTTP request handler.
#[async_trait::async_trait]
pub trait Endpoint: Send + Sync + 'static {
    /// Represents the response of the endpoint.
    type Output: IntoResponse;

    /// Get the response to the request.
    async fn call(&self, req: Request) -> Self::Output;
}

struct FnEndpoint<F>(F);

#[async_trait::async_trait]
impl<F, R> Endpoint for FnEndpoint<F>
where
    F: Fn(Request) -> R + Send + Sync + 'static,
    R: IntoResponse,
{
    type Output = R;

    async fn call(&self, req: Request) -> Self::Output {
        (self.0)(req)
    }
}

/// Create an endpoint with a function.
///
/// # Example
///
/// ```
/// use poem::{fn_endpoint, http::Method, Endpoint, Request};
///
/// let ep = fn_endpoint(|req| req.method().to_string());
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = ep
///     .call(Request::builder().method(Method::GET).finish())
///     .await;
/// assert_eq!(resp, "GET");
/// # });
/// ```
pub fn fn_endpoint<F, R>(f: F) -> impl Endpoint<Output = R>
where
    F: Fn(Request) -> R + Send + Sync + 'static,
    R: IntoResponse,
{
    FnEndpoint(f)
}

#[async_trait::async_trait]
impl<T: Endpoint + ?Sized> Endpoint for Box<T> {
    type Output = T::Output;

    async fn call(&self, req: Request) -> Self::Output {
        self.as_ref().call(req).await
    }
}

#[async_trait::async_trait]
impl<T: Endpoint + ?Sized> Endpoint for Arc<T> {
    type Output = T::Output;

    async fn call(&self, req: Request) -> Self::Output {
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
    /// let mut app = route();
    /// app.at("/").get(index.with(AddData::new(100i32)));
    /// ```
    fn with<T>(self, middleware: T) -> T::Output
    where
        T: Middleware<Self>,
        Self: Sized,
    {
        middleware.transform(self)
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

    /// Maps the output of this endpoint.
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
    /// assert_eq!(resp, "abcdef");
    /// # });
    fn after<F, Fut, R>(self, f: F) -> After<Self, F>
    where
        F: Fn(Self::Output) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: IntoResponse,
        Self: Sized,
    {
        After::new(self, f)
    }

    /// Convert the output of this endpoint into a response.
    /// [`Response`](crate::Response).
    fn map_to_response(self) -> MapToResponse<Self>
    where
        Self: Sized,
    {
        MapToResponse::new(self)
    }

    /// Convert the output of this endpoint into a result `Result<Response>`.
    /// [`Response`](crate::Response).
    fn map_to_result(self) -> MapToResult<Self>
    where
        Self: Sized,
    {
        MapToResult::new(self)
    }

    /// Calls `f` if the result is `Ok`, otherwise returns the `Err` value of
    /// self.
    fn and_then<F, Fut, R, R2>(self, f: F) -> AndThen<Self, F>
    where
        F: Fn(R) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<R2>> + Send + 'static,
        R: IntoResponse,
        R2: IntoResponse,
        Self: Endpoint<Output = Result<R>> + Sized,
    {
        AndThen::new(self, f)
    }

    /// Maps the response of this endpoint.
    fn map_ok<F, Fut, R, R2>(self, f: F) -> MapOk<Self, F>
    where
        F: Fn(R) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R2> + Send + 'static,
        R: IntoResponse,
        R2: IntoResponse,
        Self: Endpoint<Output = Result<R>> + Sized,
    {
        MapOk::new(self, f)
    }

    /// Maps the error of this endpoint.
    fn map_err<F, Fut, R>(self, f: F) -> MapErr<Self, F>
    where
        F: Fn(Error) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Error> + Send + 'static,
        R: IntoResponse,
        Self: Endpoint<Output = Result<R>> + Sized,
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

    #[tokio::test]
    async fn test_before() {
        assert_eq!(
            fn_endpoint(|req| req.method().to_string())
                .before(|mut req| async move {
                    req.set_method(Method::POST);
                    req
                })
                .call(Request::default())
                .await,
            "POST"
        );
    }

    #[tokio::test]
    async fn test_after() {
        assert_eq!(
            fn_endpoint(|_| "abc")
                .after(|_| async { "def" })
                .call(Request::default())
                .await,
            "def"
        );
    }

    #[tokio::test]
    async fn test_map_to_result() {
        assert_eq!(
            fn_endpoint(|_| Response::builder().status(StatusCode::OK).body("abc"))
                .map_to_result()
                .call(Request::default())
                .await
                .unwrap()
                .take_body()
                .into_string()
                .await
                .unwrap(),
            "abc"
        );

        assert_eq!(
            fn_endpoint(|_| Response::builder().status(StatusCode::BAD_REQUEST).finish())
                .map_to_result()
                .call(Request::default())
                .await
                .unwrap_err(),
            Error::status(StatusCode::BAD_REQUEST)
        );
    }

    #[tokio::test]
    async fn test_map_to_response() {
        assert_eq!(
            fn_endpoint(|_| Ok::<_, Error>("abc"))
                .map_to_response()
                .call(Request::default())
                .await
                .take_body()
                .into_string()
                .await
                .unwrap(),
            "abc"
        );

        assert_eq!(
            fn_endpoint(|_| Err::<(), Error>(Error::status(StatusCode::BAD_REQUEST)))
                .map_to_response()
                .call(Request::default())
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn test_and_then() {
        assert_eq!(
            fn_endpoint(|_| Ok("abc"))
                .and_then(|resp| async move { Ok(resp.to_string() + "def") })
                .call(Request::default())
                .await
                .unwrap(),
            "abcdef"
        );

        assert_eq!(
            fn_endpoint(|_| Err::<String, _>(Error::status(StatusCode::BAD_REQUEST)))
                .and_then(|resp| async move { Ok(resp + "def") })
                .call(Request::default())
                .await
                .unwrap_err(),
            Error::status(StatusCode::BAD_REQUEST)
        );
    }

    #[tokio::test]
    async fn test_map_ok() {
        assert_eq!(
            fn_endpoint(|_| Ok("abc"))
                .map_ok(|resp| async move { resp.to_string() + "def" })
                .call(Request::default())
                .await
                .unwrap(),
            "abcdef"
        );

        assert_eq!(
            fn_endpoint(|_| Err::<String, Error>(Error::status(StatusCode::BAD_REQUEST)))
                .map_ok(|resp| async move { resp.to_string() + "def" })
                .call(Request::default())
                .await
                .unwrap_err(),
            Error::status(StatusCode::BAD_REQUEST)
        );
    }

    #[tokio::test]
    async fn test_map_err() {
        assert_eq!(
            fn_endpoint(|_| Ok("abc"))
                .map_err(|_| async move { Error::status(StatusCode::BAD_GATEWAY) })
                .call(Request::default())
                .await
                .unwrap(),
            "abc"
        );

        assert_eq!(
            fn_endpoint(|_| Err::<String, Error>(Error::status(StatusCode::BAD_REQUEST)))
                .map_err(|_| async move { Error::status(StatusCode::BAD_GATEWAY) })
                .call(Request::default())
                .await
                .unwrap_err(),
            Error::status(StatusCode::BAD_GATEWAY)
        );
    }
}
