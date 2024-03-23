use std::future::Future;

use crate::{status::Status, streaming::Streaming, Request, Response};

/// Represent a GRPC service
pub trait Service {
    /// The name of the GRPC service
    const NAME: &'static str;
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
