use std::future::Future;

use crate::{MethodType, Request, Response, status::Status, streaming::Streaming};

/// Represent a GRPC service
pub trait Service {
    /// The name of the GRPC service
    const NAME: &'static str;

    /// The methods of the GRPC service, as `(path, method type)` pairs where
    /// `path` is `/{NAME}/{method}`.
    ///
    /// [`RouteGrpc`](crate::RouteGrpc) looks the [`MethodType`] up here.
    /// Generated servers implement it. It defaults to empty, so a hand-written
    /// `Service` wrapping a generated server must forward it
    /// (`const METHODS: ... = <Inner as Service>::METHODS;`), otherwise its
    /// requests carry no `MethodType` in `RouteGrpc` middlewares.
    const METHODS: &'static [(&'static str, MethodType)] = &[];
}

pub trait UnaryService<R> {
    type Response;

    fn call(
        &self,
        request: Request<R>,
    ) -> impl Future<Output = Result<Response<Self::Response>, Status>> + Send;
}

pub trait ClientStreamingService<R> {
    type Response;

    fn call(
        &self,
        request: Request<Streaming<R>>,
    ) -> impl Future<Output = Result<Response<Self::Response>, Status>> + Send;
}

pub trait ServerStreamingService<R> {
    type Response;

    fn call(
        &self,
        request: Request<R>,
    ) -> impl Future<Output = Result<Response<Streaming<Self::Response>>, Status>> + Send;
}

pub trait BidirectionalStreamingService<R> {
    type Response;

    fn call(
        &self,
        request: Request<Streaming<R>>,
    ) -> impl Future<Output = Result<Response<Streaming<Self::Response>>, Status>> + Send;
}
