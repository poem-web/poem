use futures_util::StreamExt;
use http::HeaderValue;
use poem::{Body, Request, Response};

use crate::{
    Code, CompressionEncoding, Metadata, MethodType, Request as GrpcRequest,
    Response as GrpcResponse, Status, Streaming,
    codec::Codec,
    compression::get_incoming_encodings,
    encoding::{create_decode_request_body, create_encode_response_body},
    service::{
        BidirectionalStreamingService, ClientStreamingService, ServerStreamingService, UnaryService,
    },
};

/// Tags the request with the [`MethodType`] of the method it is routed to.
/// `methods` holds paths relative to the service, i.e. `/{method}`.
#[doc(hidden)]
pub fn tag_method_type(req: &mut Request, methods: &[(&str, MethodType)]) {
    let path = req.uri().path();
    let method_type = methods
        .iter()
        .find(|(method_path, _)| *method_path == path)
        .map(|(_, method_type)| *method_type);
    crate::method_type::set_method_type(req, method_type);
}

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

        let res = match read_single_message(&mut stream).await {
            Ok(message) => {
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
            Err(status) => Err(status),
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

        let res = match read_single_message(&mut stream).await {
            Ok(message) => {
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
            Err(status) => Err(status),
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

/// Reads the only message of a non-client-streaming request, up to the end of
/// the request stream: responding before it ends makes the client's final,
/// empty DATA frame arrive on a closed stream, which h2 counts as a reset and
/// closes the connection after too many of them.
async fn read_single_message<T>(stream: &mut Streaming<T>) -> Result<T, Status> {
    let message = stream
        .next()
        .await
        .ok_or_else(|| Status::new(Code::Internal).with_message("missing request message"))??;
    match stream.next().await {
        Some(Ok(_)) => {
            Err(Status::new(Code::Internal).with_message("expected only one request message"))
        }
        Some(Err(status)) => Err(status),
        None => Ok(message),
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

#[cfg(test)]
mod tests {
    use std::{
        io::{Error as IoError, Result as IoResult},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    use bytes::{Buf, BufMut, BytesMut};
    use futures_util::TryStreamExt;
    use poem::{
        Server,
        listener::{Acceptor, Listener, TcpListener},
    };

    use super::*;
    use crate::{
        ClientConfig,
        client::GrpcClient,
        codec::{Decoder, Encoder},
    };

    /// Encodes an `i32` as 4 big-endian bytes, and fails on a negative one.
    #[derive(Default)]
    struct I32Codec;

    impl Codec for I32Codec {
        const CONTENT_TYPES: &'static [&'static str] = &["application/grpc"];

        type Encode = i32;
        type Decode = i32;
        type Encoder = I32Codec;
        type Decoder = I32Codec;

        fn encoder(&mut self) -> Self::Encoder {
            I32Codec
        }

        fn decoder(&mut self) -> Self::Decoder {
            I32Codec
        }
    }

    impl Encoder for I32Codec {
        type Item = i32;

        fn encode(&mut self, message: i32, buf: &mut BytesMut) -> IoResult<()> {
            if message < 0 {
                return Err(IoError::other("negative"));
            }
            buf.put_i32(message);
            Ok(())
        }
    }

    impl Decoder for I32Codec {
        type Item = i32;

        fn decode(&mut self, mut buf: &[u8]) -> IoResult<i32> {
            Ok(buf.get_i32())
        }
    }

    /// Responds to a unary call with `10 - request`, which fails to encode
    /// for a request above 10.
    struct Subtract;

    impl UnaryService<i32> for Subtract {
        type Response = i32;

        async fn call(&self, request: GrpcRequest<i32>) -> Result<GrpcResponse<i32>, Status> {
            Ok(GrpcResponse::new(10 - request.into_inner()))
        }
    }

    /// Responds to a server-streaming call with `items`, then sets `drained`
    /// once its response stream was polled to its end.
    #[derive(Clone)]
    struct Items {
        items: Vec<Result<i32, Status>>,
        drained: Arc<AtomicBool>,
    }

    impl ServerStreamingService<i32> for Items {
        type Response = i32;

        async fn call(
            &self,
            _request: GrpcRequest<i32>,
        ) -> Result<GrpcResponse<Streaming<i32>>, Status> {
            let drained = self.drained.clone();
            let end = futures_util::stream::poll_fn(move |_| {
                drained.store(true, Ordering::SeqCst);
                std::task::Poll::Ready(None)
            });
            Ok(GrpcResponse::new_streaming(
                futures_util::stream::iter(self.items.clone()).chain(end),
            ))
        }
    }

    /// Serves `Subtract` at `/unary` and `items` at `/stream` over HTTP/2, and
    /// returns a client for it and the flag set when `items` was read to its
    /// end.
    async fn serve(items: Vec<Result<i32, Status>>) -> (GrpcClient, Arc<AtomicBool>) {
        let drained = Arc::new(AtomicBool::new(false));
        let items = Items {
            items,
            drained: drained.clone(),
        };
        let app = poem::Route::new()
            .at(
                "/unary",
                poem::endpoint::make(|req| async move {
                    GrpcServer::new(I32Codec, None, &[])
                        .unary(Subtract, req)
                        .await
                }),
            )
            .at(
                "/stream",
                poem::endpoint::make(move |req| {
                    let items = items.clone();
                    async move {
                        GrpcServer::new(I32Codec, None, &[])
                            .server_streaming(items, req)
                            .await
                    }
                }),
            );
        let acceptor = TcpListener::bind("127.0.0.1:0")
            .into_acceptor()
            .await
            .unwrap();
        let addr = *acceptor.local_addr()[0].as_socket_addr().unwrap();
        tokio::spawn(Server::new_with_acceptor(acceptor).run(app));
        let cli = GrpcClient::new(
            ClientConfig::builder()
                .uri(format!("http://{addr}"))
                .build()
                .unwrap(),
        );
        (cli, drained)
    }

    /// Calls `/stream` and returns the messages received before the stream's
    /// end, and the status it ended with, if not OK.
    async fn stream(cli: &GrpcClient) -> (Vec<i32>, Option<Status>) {
        let mut stream = cli
            .server_streaming("/stream", I32Codec, GrpcRequest::new(0))
            .await
            .unwrap()
            .into_inner();
        let mut messages = vec![];
        loop {
            match stream.try_next().await {
                Ok(Some(message)) => messages.push(message),
                Ok(None) => return (messages, None),
                Err(status) => return (messages, Some(status)),
            }
        }
    }

    #[tokio::test]
    async fn responses_without_errors() {
        let (cli, drained) = serve(vec![Ok(1), Ok(2)]).await;
        let resp = cli.unary("/unary", I32Codec, GrpcRequest::new(3)).await;
        assert_eq!(resp.unwrap().into_inner(), 7);
        let (messages, status) = stream(&cli).await;
        assert_eq!(messages, [1, 2]);
        assert!(status.is_none());
        assert!(drained.load(Ordering::SeqCst));
    }

    /// A unary response that fails to encode fails the call with that error,
    /// instead of ending without a message and with an OK status.
    #[tokio::test]
    async fn unary_response_fails_to_encode() {
        let (cli, _) = serve(vec![]).await;
        let status = cli
            .unary("/unary", I32Codec, GrpcRequest::new(11))
            .await
            .unwrap_err();
        assert_eq!(status.code(), Code::Internal);
        assert_eq!(status.message(), Some("negative"));
    }

    /// A streamed message that fails to encode ends the response with that
    /// error, instead of being left out of an otherwise successful response.
    #[tokio::test]
    async fn streamed_response_fails_to_encode() {
        let (cli, drained) = serve(vec![Ok(1), Ok(-1), Ok(2)]).await;
        let (messages, status) = stream(&cli).await;
        assert_eq!(messages, [1]);
        let status = status.expect("the response ended with an OK status");
        assert_eq!(status.code(), Code::Internal);
        assert_eq!(status.message(), Some("negative"));
        assert!(!drained.load(Ordering::SeqCst));
    }

    /// An error from the handler's response stream reaches the client as the
    /// status of the response, after the messages before it. (This held
    /// before the encoder stopped at the error itself, since hyper sends
    /// nothing after the first trailers.)
    #[tokio::test]
    async fn streamed_response_error() {
        let error = Status::new(Code::NotFound).with_message("gone");
        let (cli, _) = serve(vec![Ok(1), Err(error), Ok(2)]).await;
        let (messages, status) = stream(&cli).await;
        assert_eq!(messages, [1]);
        let status = status.unwrap();
        assert_eq!(status.code(), Code::NotFound);
        assert_eq!(status.message(), Some("gone"));
    }
}
