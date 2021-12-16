use std::{future::Future, marker::PhantomData, sync::Arc};

use super::{
    After, AndThen, Around, Before, CatchError, InspectError, InspectTypedError, Map, MapToResponse,
};
use crate::{
    error::IntoResult,
    middleware::{AddData, AddDataEndpoint},
    Error, IntoResponse, Middleware, Request, Response, Result,
};

/// An HTTP request handler.
#[async_trait::async_trait]
pub trait Endpoint: Send + Sync {
    /// Represents the response of the endpoint.
    type Output: IntoResponse;

    /// Get the response to the request.
    async fn call(&self, req: Request) -> Result<Self::Output>;
}

struct SyncFnEndpoint<T, F> {
    _mark: PhantomData<T>,
    f: F,
}

#[async_trait::async_trait]
impl<F, T, R> Endpoint for SyncFnEndpoint<T, F>
where
    F: Fn(Request) -> R + Send + Sync,
    T: IntoResponse + Sync,
    R: IntoResult<T>,
{
    type Output = T;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        (self.f)(req).into_result()
    }
}

struct AsyncFnEndpoint<T, F> {
    _mark: PhantomData<T>,
    f: F,
}

#[async_trait::async_trait]
impl<F, Fut, T, R> Endpoint for AsyncFnEndpoint<T, F>
where
    F: Fn(Request) -> Fut + Sync + Send,
    Fut: Future<Output = R> + Send,
    T: IntoResponse + Sync,
    R: IntoResult<T>,
{
    type Output = T;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        (self.f)(req).await.into_result()
    }
}

/// Combines two different endpoints for [`Endpoint::with_if`].
pub enum EitherEndpoint<A, B> {
    A(A),
    B(B),
}

#[async_trait::async_trait]
impl<A, B> Endpoint for EitherEndpoint<A, B>
where
    A: Endpoint,
    B: Endpoint,
{
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        match self {
            EitherEndpoint::A(a) => a.call(req).await.map(IntoResponse::into_response),
            EitherEndpoint::B(b) => b.call(req).await.map(IntoResponse::into_response),
        }
    }
}

/// Create an endpoint with a function.
///
/// The output can be any type that implements [`IntoResult`].
///
/// # Example
///
/// ```
/// use poem::{endpoint::make_sync, http::Method, Endpoint, Request};
///
/// let ep = make_sync(|req| req.method().to_string());
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = ep
///     .call(Request::builder().method(Method::GET).finish())
///     .await
///     .unwrap();
/// assert_eq!(resp, "GET");
/// # });
/// ```
pub fn make_sync<F, T, R>(f: F) -> impl Endpoint<Output = T>
where
    F: Fn(Request) -> R + Send + Sync,
    T: IntoResponse + Sync,
    R: IntoResult<T>,
{
    SyncFnEndpoint {
        _mark: PhantomData,
        f,
    }
}

/// Create an endpoint with a asyncness function.
///
/// The output can be any type that implements [`IntoResult`].
///
/// # Example
///
/// ```
/// use poem::{endpoint::make, http::Method, Endpoint, Request};
///
/// let ep = make(|req| async move { req.method().to_string() });
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = ep
///     .call(Request::builder().method(Method::GET).finish())
///     .await
///     .unwrap();
/// assert_eq!(resp, "GET");
/// # });
/// ```
pub fn make<F, Fut, T, R>(f: F) -> impl Endpoint<Output = T>
where
    F: Fn(Request) -> Fut + Send + Sync,
    Fut: Future<Output = R> + Send,
    T: IntoResponse + Sync,
    R: IntoResult<T>,
{
    AsyncFnEndpoint {
        _mark: PhantomData,
        f,
    }
}

#[async_trait::async_trait]
impl<T: Endpoint + ?Sized> Endpoint for &T {
    type Output = T::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        T::call(self, req).await
    }
}

#[async_trait::async_trait]
impl<T: Endpoint + ?Sized> Endpoint for Box<T> {
    type Output = T::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        self.as_ref().call(req).await
    }
}

#[async_trait::async_trait]
impl<T: Endpoint + ?Sized> Endpoint for Arc<T> {
    type Output = T::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        self.as_ref().call(req).await
    }
}

/// An owned dynamically typed `Endpoint` for use in cases where you can’t
/// statically type your result or need to add some indirection.
pub type BoxEndpoint<'a, T> = Box<dyn Endpoint<Output = T> + 'a>;

/// Extension trait for [`Endpoint`].
pub trait EndpointExt: IntoEndpoint {
    /// Wrap the endpoint in a Box.
    fn boxed<'a>(self) -> BoxEndpoint<'a, <Self::Endpoint as Endpoint>::Output>
    where
        Self: Sized + 'a,
    {
        Box::new(self.into_endpoint())
    }

    /// Use middleware to transform this endpoint.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     get, handler, http::StatusCode, middleware::AddData, web::Data, Endpoint, EndpointExt,
    ///     Request, Route,
    /// };
    ///
    /// #[handler]
    /// async fn index(Data(data): Data<&i32>) -> String {
    ///     format!("{}", data)
    /// }
    ///
    /// let app = Route::new().at("/", get(index)).with(AddData::new(100i32));
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp = app.call(Request::default()).await.unwrap();
    /// assert_eq!(resp.status(), StatusCode::OK);
    /// assert_eq!(resp.into_body().into_string().await.unwrap(), "100");
    /// # });
    /// ```
    fn with<T>(self, middleware: T) -> T::Output
    where
        T: Middleware<Self::Endpoint>,
        Self: Sized,
    {
        middleware.transform(self.into_endpoint())
    }

    /// if `enable` is `true` then use middleware to transform this endpoint.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     get, handler,
    ///     http::{StatusCode, Uri},
    ///     middleware::AddData,
    ///     web::Data,
    ///     Endpoint, EndpointExt, Request, Route,
    /// };
    ///
    /// #[handler]
    /// async fn index(data: Option<Data<&i32>>) -> String {
    ///     match data {
    ///         Some(data) => data.0.to_string(),
    ///         None => "none".to_string(),
    ///     }
    /// }
    ///
    /// let app = Route::new()
    ///     .at("/a", get(index).with_if(true, AddData::new(100i32)))
    ///     .at("/b", get(index).with_if(false, AddData::new(100i32)));
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp = app
    ///     .call(Request::builder().uri(Uri::from_static("/a")).finish())
    ///     .await
    ///     .unwrap();
    /// assert_eq!(resp.status(), StatusCode::OK);
    /// assert_eq!(resp.into_body().into_string().await.unwrap(), "100");
    ///
    /// let resp = app
    ///     .call(Request::builder().uri(Uri::from_static("/b")).finish())
    ///     .await
    ///     .unwrap();
    /// assert_eq!(resp.status(), StatusCode::OK);
    /// assert_eq!(resp.into_body().into_string().await.unwrap(), "none");
    /// # });
    /// ```
    fn with_if<T>(self, enable: bool, middleware: T) -> EitherEndpoint<Self, T::Output>
    where
        T: Middleware<Self::Endpoint>,
        Self: Sized,
    {
        if !enable {
            EitherEndpoint::A(self)
        } else {
            EitherEndpoint::B(middleware.transform(self.into_endpoint()))
        }
    }

    /// A helper function, similar to `with(AddData(T))`.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{handler, http::StatusCode, web::Data, Endpoint, EndpointExt, Request};
    ///
    /// #[handler]
    /// async fn index(data: Data<&i32>) -> String {
    ///     format!("{}", data.0)
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let mut resp = index.data(100i32).call(Request::default()).await.unwrap();
    /// assert_eq!(resp.status(), StatusCode::OK);
    /// assert_eq!(resp.take_body().into_string().await.unwrap(), "100");
    /// # });
    /// ```
    fn data<T>(self, data: T) -> AddDataEndpoint<Self::Endpoint, T>
    where
        T: Clone + Send + Sync + 'static,
        Self: Sized,
    {
        self.with(AddData::new(data))
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
    ///         Ok(req)
    ///     })
    ///     .call(Request::default())
    ///     .await
    ///     .unwrap();
    /// assert_eq!(resp.take_body().into_string().await.unwrap(), "abc");
    /// # });
    /// ```
    fn before<F, Fut>(self, f: F) -> Before<Self, F>
    where
        F: Fn(Request) -> Fut + Send + Sync,
        Fut: Future<Output = Result<Request>> + Send,
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
    ///     .after(|res| async move {
    ///         match res {
    ///             Ok(resp) => Ok(resp.into_body().into_string().await.unwrap() + "def"),
    ///             Err(err) => Err(err),
    ///         }
    ///     })
    ///     .call(Request::default())
    ///     .await
    ///     .unwrap();
    /// assert_eq!(resp, "abcdef");
    /// # });
    /// ```
    fn after<F, Fut, T>(self, f: F) -> After<Self::Endpoint, F>
    where
        F: Fn(Result<<Self::Endpoint as Endpoint>::Output>) -> Fut + Send + Sync,
        Fut: Future<Output = Result<T>> + Send,
        T: IntoResponse,
        Self: Sized,
    {
        After::new(self.into_endpoint(), f)
    }

    /// Maps the request and response of this endpoint.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     handler,
    ///     http::{HeaderMap, HeaderValue, StatusCode},
    ///     Endpoint, EndpointExt, Error, Request, Result,
    /// };
    ///
    /// #[handler]
    /// async fn index(headers: &HeaderMap) -> String {
    ///     headers
    ///         .get("x-value")
    ///         .and_then(|value| value.to_str().ok())
    ///         .unwrap()
    ///         .to_string()
    ///         + ","
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let mut resp = index
    ///     .around(|ep, mut req| async move {
    ///         req.headers_mut()
    ///             .insert("x-value", HeaderValue::from_static("hello"));
    ///         let mut resp = ep.call(req).await?;
    ///         Ok(resp.take_body().into_string().await.unwrap() + "world")
    ///     })
    ///     .call(Request::default())
    ///     .await
    ///     .unwrap();
    /// assert_eq!(resp, "hello,world");
    /// # });
    /// ```
    fn around<F, Fut, R>(self, f: F) -> Around<Self::Endpoint, F>
    where
        F: Fn(Arc<Self::Endpoint>, Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<R>> + Send + 'static,
        R: IntoResponse,
        Self: Sized,
    {
        Around::new(self.into_endpoint(), f)
    }

    /// Convert the output of this endpoint into a response.
    /// [`Response`](crate::Response).
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     endpoint::make, http::StatusCode, Endpoint, EndpointExt, Error, Request, Response, Result,
    /// };
    ///
    /// let ep1 = make(|_| async { "hello" }).map_to_response();
    /// let ep2 = make(|_| async { Err::<(), Error>(Error::from_status(StatusCode::BAD_REQUEST)) })
    ///     .map_to_response();
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp = ep1.call(Request::default()).await.unwrap();
    /// assert_eq!(resp.into_body().into_string().await.unwrap(), "hello");
    ///
    /// let err = ep2.call(Request::default()).await.unwrap_err();
    /// assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    /// # });
    /// ```
    fn map_to_response(self) -> MapToResponse<Self::Endpoint>
    where
        Self: Sized,
    {
        MapToResponse::new(self.into_endpoint())
    }

    /// Maps the response of this endpoint.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     endpoint::make, http::StatusCode, Endpoint, EndpointExt, Error, Request, Response, Result,
    /// };
    ///
    /// let ep = make(|_| async { "hello" }).map(|value| async move { format!("{}, world!", value) });
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let mut resp: String = ep.call(Request::default()).await.unwrap();
    /// assert_eq!(resp, "hello, world!");
    /// # });
    /// ```
    fn map<F, Fut, R, R2>(self, f: F) -> Map<Self::Endpoint, F>
    where
        F: Fn(R) -> Fut + Send + Sync,
        Fut: Future<Output = R2> + Send,
        R: IntoResponse,
        R2: IntoResponse,
        Self: Sized,
        Self::Endpoint: Endpoint<Output = R> + Sized,
    {
        Map::new(self.into_endpoint(), f)
    }

    /// Calls `f` if the result is `Ok`, otherwise returns the `Err` value of
    /// self.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     endpoint::make, http::StatusCode, Endpoint, EndpointExt, Error, Request, Response, Result,
    /// };
    ///
    /// let ep1 = make(|_| async { "hello" })
    ///     .and_then(|value| async move { Ok(format!("{}, world!", value)) });
    /// let ep2 = make(|_| async { Err::<String, _>(Error::from_status(StatusCode::BAD_REQUEST)) })
    ///     .and_then(|value| async move { Ok(format!("{}, world!", value)) });
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp: String = ep1.call(Request::default()).await.unwrap();
    /// assert_eq!(resp, "hello, world!");
    ///
    /// let err: Error = ep2.call(Request::default()).await.unwrap_err();
    /// assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    /// # });
    /// ```
    fn and_then<F, Fut, R, R2>(self, f: F) -> AndThen<Self::Endpoint, F>
    where
        F: Fn(R) -> Fut + Send + Sync,
        Fut: Future<Output = Result<R2>> + Send,
        R: IntoResponse,
        R2: IntoResponse,
        Self: Sized,
        Self::Endpoint: Endpoint<Output = R> + Sized,
    {
        AndThen::new(self.into_endpoint(), f)
    }

    /// Catch the specified type of error and convert it into a response.
    ///
    /// # Example
    ///
    /// ```
    /// use http::Uri;
    /// use poem::{
    ///     error::NotFoundError, handler, http::StatusCode, Endpoint, EndpointExt, Request, Response,
    ///     Route,
    /// };
    ///
    /// #[handler]
    /// async fn index() {}
    ///
    /// let app = Route::new().at("/index", index).catch_error(custom_404);
    ///
    /// async fn custom_404(_: NotFoundError) -> Response {
    ///     Response::builder()
    ///         .status(StatusCode::NOT_FOUND)
    ///         .body("custom not found")
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp = app
    ///     .call(Request::builder().uri(Uri::from_static("/abc")).finish())
    ///     .await
    ///     .unwrap();
    /// assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    /// assert_eq!(
    ///     resp.into_body().into_string().await.unwrap(),
    ///     "custom not found"
    /// );
    /// # })
    /// ```
    fn catch_error<F, Fut, ErrType>(self, f: F) -> CatchError<Self, F, ErrType>
    where
        F: Fn(ErrType) -> Fut + Send + Sync,
        Fut: Future<Output = Response> + Send,
        ErrType: std::error::Error + Send + Sync + 'static,
        Self: Sized,
    {
        CatchError::new(self, f)
    }

    /// Does something with each error.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{handler, EndpointExt, Route};
    ///
    /// #[handler]
    /// fn index() {}
    ///
    /// let app = Route::new().at("/", index).inspect_err(|err| {
    ///     println!("error: {}", err);
    /// });
    /// ```
    fn inspect_err<F>(self, f: F) -> InspectError<Self, F>
    where
        F: Fn(&Error) + Send + Sync,
        Self: Sized,
    {
        InspectError::new(self, f)
    }

    /// Does something with each specified error type.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{error::NotFoundError, handler, EndpointExt, Route};
    ///
    /// #[handler]
    /// fn index() {}
    ///
    /// let app = Route::new()
    ///     .at("/", index)
    ///     .inspect_typed_err(|err: &NotFoundError| {
    ///         println!("error: {}", err);
    ///     });
    /// ```
    fn inspect_typed_err<F, ErrType>(self, f: F) -> InspectTypedError<Self, F, ErrType>
    where
        F: Fn(&ErrType) + Send + Sync,
        ErrType: std::error::Error + Send + Sync + 'static,
        Self: Sized,
    {
        InspectTypedError::new(self, f)
    }
}

impl<T: IntoEndpoint> EndpointExt for T {}

/// Represents a type that can convert into endpoint.
pub trait IntoEndpoint {
    /// Represents the endpoint type.
    type Endpoint: Endpoint;

    /// Converts this object into endpoint.
    fn into_endpoint(self) -> Self::Endpoint;
}

impl<T: Endpoint> IntoEndpoint for T {
    type Endpoint = T;

    fn into_endpoint(self) -> Self::Endpoint {
        self
    }
}

#[cfg(test)]
mod test {
    use http::{HeaderValue, Uri};

    use crate::{
        endpoint::{make, make_sync},
        http::{Method, StatusCode},
        middleware::SetHeader,
        *,
    };

    #[tokio::test]
    async fn test_make() {
        let ep = make(|req| async move { format!("method={}", req.method()) }).map_to_response();
        let mut resp = ep
            .call(Request::builder().method(Method::DELETE).finish())
            .await
            .unwrap();
        assert_eq!(
            resp.take_body().into_string().await.unwrap(),
            "method=DELETE"
        );
    }

    #[tokio::test]
    async fn test_before() {
        assert_eq!(
            make_sync(|req| req.method().to_string())
                .before(|mut req| async move {
                    req.set_method(Method::POST);
                    Ok(req)
                })
                .call(Request::default())
                .await
                .unwrap(),
            "POST"
        );
    }

    #[tokio::test]
    async fn test_after() {
        assert_eq!(
            make_sync(|_| "abc")
                .after(|_| async { Ok::<_, Error>("def") })
                .call(Request::default())
                .await
                .unwrap(),
            "def"
        );
    }

    #[tokio::test]
    async fn test_map_to_response() {
        assert_eq!(
            make_sync(|_| "abc")
                .map_to_response()
                .call(Request::default())
                .await
                .unwrap()
                .take_body()
                .into_string()
                .await
                .unwrap(),
            "abc"
        );
    }

    #[tokio::test]
    async fn test_and_then() {
        assert_eq!(
            make_sync(|_| "abc")
                .and_then(|resp| async move { Ok(resp.to_string() + "def") })
                .call(Request::default())
                .await
                .unwrap(),
            "abcdef"
        );

        let err = make_sync(|_| Err::<String, _>(Error::from_status(StatusCode::BAD_REQUEST)))
            .and_then(|resp| async move { Ok(resp + "def") })
            .call(Request::default())
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_map() {
        assert_eq!(
            make_sync(|_| "abc")
                .map(|resp| async move { resp.to_string() + "def" })
                .call(Request::default())
                .await
                .unwrap(),
            "abcdef"
        );

        let err = make_sync(|_| Err::<String, _>(Error::from_status(StatusCode::BAD_REQUEST)))
            .map(|resp| async move { resp.to_string() + "def" })
            .call(Request::default())
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_around() {
        let ep = make(|req| async move { req.into_body().into_string().await.unwrap() + "b" });
        assert_eq!(
            ep.around(|ep, mut req| async move {
                req.set_body("a");
                let resp = ep.call(req).await?;
                Ok(resp + "c")
            })
            .call(Request::default())
            .await
            .unwrap(),
            "abc"
        );
    }

    #[tokio::test]
    async fn test_with_if() {
        let resp = make_sync(|_| ())
            .with_if(true, SetHeader::new().appending("a", 1))
            .call(Request::default())
            .await
            .unwrap();
        assert_eq!(
            resp.headers().get("a"),
            Some(&HeaderValue::from_static("1"))
        );

        let resp = make_sync(|_| ())
            .with_if(false, SetHeader::new().appending("a", 1))
            .call(Request::default())
            .await
            .unwrap();
        assert_eq!(resp.headers().get("a"), None);
    }

    #[tokio::test]
    async fn test_into_endpoint() {
        struct MyEndpointFactory;

        impl IntoEndpoint for MyEndpointFactory {
            type Endpoint = Route;

            fn into_endpoint(self) -> Self::Endpoint {
                Route::new()
                    .at("/a", get(make_sync(|_| "a")))
                    .at("/b", get(make_sync(|_| "b")))
            }
        }

        let app = Route::new().nest("/api", MyEndpointFactory);

        assert_eq!(
            app.call(Request::builder().uri(Uri::from_static("/api/a")).finish())
                .await
                .unwrap()
                .take_body()
                .into_string()
                .await
                .unwrap(),
            "a"
        );

        assert_eq!(
            app.call(Request::builder().uri(Uri::from_static("/api/b")).finish())
                .await
                .unwrap()
                .take_body()
                .into_string()
                .await
                .unwrap(),
            "b"
        );
    }
}
