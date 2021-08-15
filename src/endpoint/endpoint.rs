use std::sync::Arc;

use super::{AndThen, Before, Map, MapErr, MapOk};
use crate::{error::Result, middleware::Middleware, request::Request, response::Response};

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
    /// use poem::web::Data;
    /// use poem::middleware::AddData;
    /// use poem::prelude::*;
    ///
    /// async fn index(Data(data): Data<i32>) -> String {
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
    fn before<F>(self, f: F) -> Before<Self, F>
    where
        F: Fn(Request) -> Result<Request>,
        Self: Sized,
    {
        Before::new(self, f)
    }

    /// Maps the result of this endpoint.
    fn map<F>(self, f: F) -> Map<Self, F>
    where
        F: Fn(Result<Response>) -> Result<Response>,
        Self: Sized,
    {
        Map::new(self, f)
    }

    /// calls `f` if the result is `Ok`, otherwise returns the `Err` value of
    /// self.
    fn and_then<F>(self, f: F) -> AndThen<Self, F>
    where
        F: Fn(Response) -> Result<Response>,
        Self: Sized,
    {
        AndThen::new(self, f)
    }

    /// Maps the response of this endpoint.
    fn map_ok<F>(self, f: F) -> MapOk<Self, F>
    where
        F: Fn(Response) -> Response,
        Self: Sized,
    {
        MapOk::new(self, f)
    }

    /// Maps the error of this endpoint.
    fn map_err<F>(self, f: F) -> MapErr<Self, F>
    where
        F: Fn(Response) -> Response,
        Self: Sized,
    {
        MapErr::new(self, f)
    }
}

impl<T: Endpoint> EndpointExt for T {}
