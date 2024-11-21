use futures_util::StreamExt;
use http::HeaderValue;
use poem::{Body, Request, Response};

use crate::{
    codec::Codec,
    compression::get_incoming_encodings,
    encoding::{create_decode_request_body, create_encode_response_body},
    service::{
        BidirectionalStreamingService, ClientStreamingService, ServerStreamingService, UnaryService,
    },
    Code, CompressionEncoding, Metadata, Request as GrpcRequest, Response as GrpcResponse, Status,
    Streaming,
};

#[doc(hidden)]
pub struct GrpcServer<'a, T> {
    codec: T,
    send_compressed: Option<CompressionEncoding>,
    accept_compressed: &'a [CompressionEncoding],
}

impl<'a, T: Codec> GrpcServer<'a, T> {
    #[inline]
    pub fn new(
        codec: T,
        send_compressed: Option<CompressionEncoding>,
        accept_compressed: &'a [CompressionEncoding],
    ) -> Self {
        Self {
            codec,
            send_compressed,
            accept_compressed,
        }
    }

    pub async fn unary<S>(mut self, service: S, request: Request) -> Response
    where
        S: UnaryService<T::Decode, Response = T::Encode>,
    {
        let (parts, body) = request.into_parts();
        let mut resp = Response::default().set_content_type(T::CONTENT_TYPES[0]);
        let incoming_encoding = match get_incoming_encodings(&parts.headers, self.accept_compressed)
        {
            Ok(incoming_encoding) => incoming_encoding,
            Err(status) => {
                resp.headers_mut().extend(status.to_headers());
                return resp;
            }
        };
        let mut stream = create_decode_request_body(self.codec.decoder(), body, incoming_encoding);

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

        match res {
            Ok(grpc_resp) => {
                let GrpcResponse { metadata, message } = grpc_resp;
                let body = create_encode_response_body(
                    self.codec.encoder(),
                    Streaming::new(futures_util::stream::once(async move { Ok(message) })),
                    self.send_compressed,
                );
                update_http_response(&mut resp, metadata, body, self.send_compressed);
            }
            Err(status) => resp.headers_mut().extend(status.to_headers()),
        }

        resp
    }

    pub async fn client_streaming<S>(mut self, service: S, request: Request) -> Response
    where
        S: ClientStreamingService<T::Decode, Response = T::Encode>,
    {
        let (parts, body) = request.into_parts();
        let mut resp = Response::default().set_content_type(T::CONTENT_TYPES[0]);
        let incoming_encoding = match get_incoming_encodings(&parts.headers, self.accept_compressed)
        {
            Ok(incoming_encoding) => incoming_encoding,
            Err(status) => {
                resp.headers_mut().extend(status.to_headers());
                return resp;
            }
        };
        let stream = create_decode_request_body(self.codec.decoder(), body, incoming_encoding);

        let res = service
            .call(GrpcRequest {
                metadata: Metadata {
                    headers: parts.headers,
                },
                extensions: parts.extensions,
                message: stream,
            })
            .await;

        match res {
            Ok(grpc_resp) => {
                let GrpcResponse { metadata, message } = grpc_resp;
                let body = create_encode_response_body(
                    self.codec.encoder(),
                    Streaming::new(futures_util::stream::once(async move { Ok(message) })),
                    self.send_compressed,
                );
                update_http_response(&mut resp, metadata, body, self.send_compressed);
            }
            Err(status) => {
                resp.headers_mut().extend(status.to_headers());
            }
        }

        resp
    }

    pub async fn server_streaming<S>(mut self, service: S, request: Request) -> Response
    where
        S: ServerStreamingService<T::Decode, Response = T::Encode>,
    {
        let (parts, body) = request.into_parts();
        let mut resp = Response::default().set_content_type(T::CONTENT_TYPES[0]);
        let incoming_encoding = match get_incoming_encodings(&parts.headers, self.accept_compressed)
        {
            Ok(incoming_encoding) => incoming_encoding,
            Err(status) => {
                resp.headers_mut().extend(status.to_headers());
                return resp;
            }
        };
        let mut stream = create_decode_request_body(self.codec.decoder(), body, incoming_encoding);

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

        match res {
            Ok(grpc_resp) => {
                let GrpcResponse { metadata, message } = grpc_resp;
                let body = create_encode_response_body(
                    self.codec.encoder(),
                    message,
                    self.send_compressed,
                );
                update_http_response(&mut resp, metadata, body, self.send_compressed);
            }
            Err(status) => {
                resp.headers_mut().extend(status.to_headers());
            }
        }

        resp
    }

    pub async fn bidirectional_streaming<S>(mut self, service: S, request: Request) -> Response
    where
        S: BidirectionalStreamingService<T::Decode, Response = T::Encode>,
    {
        let (parts, body) = request.into_parts();
        let mut resp = Response::default().set_content_type(T::CONTENT_TYPES[0]);
        let incoming_encoding = match get_incoming_encodings(&parts.headers, self.accept_compressed)
        {
            Ok(incoming_encoding) => incoming_encoding,
            Err(status) => {
                resp.headers_mut().extend(status.to_headers());
                return resp;
            }
        };
        let stream = create_decode_request_body(self.codec.decoder(), body, incoming_encoding);

        let res = service
            .call(GrpcRequest {
                metadata: Metadata {
                    headers: parts.headers,
                },
                message: stream,
                extensions: parts.extensions,
            })
            .await;

        match res {
            Ok(grpc_resp) => {
                let GrpcResponse { metadata, message } = grpc_resp;
                let body = create_encode_response_body(
                    self.codec.encoder(),
                    message,
                    self.send_compressed,
                );
                update_http_response(&mut resp, metadata, body, self.send_compressed);
            }
            Err(status) => {
                resp.headers_mut().extend(status.to_headers());
            }
        }

        resp
    }
}

fn update_http_response(
    resp: &mut Response,
    metadata: Metadata,
    body: Body,
    send_compressed: Option<CompressionEncoding>,
) {
    resp.headers_mut().extend(metadata.headers);
    if let Some(send_compressed) = send_compressed {
        resp.headers_mut().insert(
            "grpc-encoding",
            HeaderValue::from_str(send_compressed.as_str()).expect("BUG: invalid encoding"),
        );
    }
    resp.set_body(body);
}
