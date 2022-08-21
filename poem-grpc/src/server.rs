use futures_util::StreamExt;
use poem::{Request, Response};

use crate::{
    codec::Codec,
    encoding::{create_decode_request_body, create_encode_response_body},
    service::{
        BidirectionalStreamingService, ClientStreamingService, ServerStreamingService, UnaryService,
    },
    Code, Metadata, Request as GrpcRequest, Response as GrpcResponse, Status, Streaming,
};

#[doc(hidden)]
pub struct GrpcServer<T> {
    codec: T,
}

impl<T: Codec> GrpcServer<T> {
    #[inline]
    pub fn new(codec: T) -> Self {
        Self { codec }
    }

    pub async fn unary<S>(&mut self, service: S, request: Request) -> Response
    where
        S: UnaryService<T::Decode, Response = T::Encode>,
    {
        let (parts, body) = request.into_parts();
        let mut stream = create_decode_request_body(self.codec.decoder(), body);

        let res = match stream.next().await {
            Some(Ok(message)) => {
                service
                    .call(GrpcRequest {
                        metadata: Metadata {
                            headers: parts.headers,
                        },
                        message,
                        extensions: parts.extensions,
                    })
                    .await
            }
            Some(Err(status)) => Err(status),
            None => Err(Status::new(Code::Internal).with_message("missing request message")),
        };

        let mut resp = Response::default();

        match res {
            Ok(grpc_resp) => {
                let GrpcResponse { metadata, message } = grpc_resp;
                let body = create_encode_response_body(
                    self.codec.encoder(),
                    Streaming::new(futures_util::stream::once(async move { Ok(message) })),
                );
                resp.headers_mut().extend(metadata.headers);
                resp.set_body(body);
            }
            Err(status) => {
                *resp.headers_mut() = status.to_headers();
            }
        }

        resp
    }

    pub async fn client_streaming<S>(&mut self, service: S, request: Request) -> Response
    where
        S: ClientStreamingService<T::Decode, Response = T::Encode>,
    {
        let (parts, body) = request.into_parts();
        let stream = create_decode_request_body(self.codec.decoder(), body);

        let res = service
            .call(GrpcRequest {
                metadata: Metadata {
                    headers: parts.headers,
                },
                extensions: parts.extensions,
                message: stream,
            })
            .await;

        let mut resp = Response::default();

        match res {
            Ok(grpc_resp) => {
                let GrpcResponse { metadata, message } = grpc_resp;
                let body = create_encode_response_body(
                    self.codec.encoder(),
                    Streaming::new(futures_util::stream::once(async move { Ok(message) })),
                );
                resp.headers_mut().extend(metadata.headers);
                resp.set_body(body);
            }
            Err(status) => {
                *resp.headers_mut() = status.to_headers();
            }
        }

        resp
    }

    pub async fn server_streaming<S>(&mut self, service: S, request: Request) -> Response
    where
        S: ServerStreamingService<T::Decode, Response = T::Encode>,
    {
        let (parts, body) = request.into_parts();
        let mut stream = create_decode_request_body(self.codec.decoder(), body);

        let res = match stream.next().await {
            Some(Ok(message)) => {
                service
                    .call(GrpcRequest {
                        metadata: Metadata {
                            headers: parts.headers,
                        },
                        message,
                        extensions: parts.extensions,
                    })
                    .await
            }
            Some(Err(status)) => Err(status),
            None => Err(Status::new(Code::Internal).with_message("missing request message")),
        };

        let mut resp = Response::default();

        match res {
            Ok(grpc_resp) => {
                let GrpcResponse { metadata, message } = grpc_resp;
                let body = create_encode_response_body(self.codec.encoder(), message);
                resp.headers_mut().extend(metadata.headers);
                resp.set_body(body);
            }
            Err(status) => {
                *resp.headers_mut() = status.to_headers();
            }
        }

        resp
    }

    pub async fn bidirectional_streaming<S>(&mut self, service: S, request: Request) -> Response
    where
        S: BidirectionalStreamingService<T::Decode, Response = T::Encode>,
    {
        let (parts, body) = request.into_parts();
        let stream = create_decode_request_body(self.codec.decoder(), body);

        let res = service
            .call(GrpcRequest {
                metadata: Metadata {
                    headers: parts.headers,
                },
                message: stream,
                extensions: parts.extensions,
            })
            .await;

        let mut resp = Response::default();

        match res {
            Ok(grpc_resp) => {
                let GrpcResponse { metadata, message } = grpc_resp;
                let body = create_encode_response_body(self.codec.encoder(), message);
                resp.headers_mut().extend(metadata.headers);
                resp.set_body(body);
            }
            Err(status) => {
                *resp.headers_mut() = status.to_headers();
            }
        }

        resp
    }
}
