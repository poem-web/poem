use crate::{status::Status, streaming::Streaming, Request, Response};

/// Represent a GRPC service
pub trait Service {
    /// The name of the GRPC service
    const NAME: &'static str;
}

#[poem::async_trait]
pub trait UnaryService<R> {
    type Response;

    async fn call(&self, request: Request<R>) -> Result<Response<Self::Response>, Status>;
}

#[poem::async_trait]
pub trait ClientStreamingService<R> {
    type Response;

    async fn call(
        &self,
        request: Request<Streaming<R>>,
    ) -> Result<Response<Self::Response>, Status>;
}

#[poem::async_trait]
pub trait ServerStreamingService<R> {
    type Response;

    async fn call(
        &self,
        request: Request<R>,
    ) -> Result<Response<Streaming<Self::Response>>, Status>;
}

#[poem::async_trait]
pub trait BidirectionalStreamingService<R> {
    type Response;

    async fn call(
        &self,
        request: Request<Streaming<R>>,
    ) -> Result<Response<Streaming<Self::Response>>, Status>;
}
