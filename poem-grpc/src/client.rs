use std::{io::Error as IoError, sync::Arc};

use bytes::Bytes;
use http_body_util::BodyExt;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use poem::{
    Endpoint, EndpointExt, IntoEndpoint, Middleware, Request as HttpRequest,
    Response as HttpResponse,
    endpoint::{DynEndpoint, ToDynEndpoint},
    http::{
        Extensions, HeaderValue, Method, StatusCode, Uri, Version,
        header::{self, InvalidHeaderValue},
        uri::InvalidUri,
    },
};
use rustls::ClientConfig as TlsClientConfig;

use crate::{
    Code, CompressionEncoding, Metadata, Request, Response, Status, Streaming,
    codec::Codec,
    compression::get_incoming_encodings,
    connector::HttpsConnector,
    encoding::{
        create_decode_response_body, create_encode_request_body, create_encode_unary_request_body,
        decode_single_response_message, discard_body,
    },
};

pub(crate) type BoxBody = http_body_util::combinators::BoxBody<Bytes, IoError>;

/// A configuration for GRPC client
pub struct ClientConfig {
    uris: Vec<Uri>,
    origin: Option<Uri>,
    user_agent: Option<HeaderValue>,
    tls_config: Option<TlsClientConfig>,
    max_header_list_size: u32,
}

impl ClientConfig {
    /// Create a `ClientConfig` builder
    pub fn builder() -> ClientConfigBuilder {
        ClientConfigBuilder {
            config: Ok(ClientConfig {
                uris: vec![],
                origin: None,
                user_agent: None,
                tls_config: None,
                max_header_list_size: 16384,
            }),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClientBuilderError {
    /// Invalid uri
    #[error("invalid uri: {0}")]
    InvalidUri(InvalidUri),

    /// Invalid origin
    #[error("invalid origin: {0}")]
    InvalidOrigin(InvalidUri),

    /// Invalid user-agent
    #[error("invalid user-agent: {0}")]
    InvalidUserAgent(InvalidHeaderValue),
}

/// A `ClientConfig` builder
pub struct ClientConfigBuilder {
    config: Result<ClientConfig, ClientBuilderError>,
}

impl ClientConfigBuilder {
    /// Add a uri as GRPC endpoint
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use poem_grpc::ClientConfig;
    /// let cfg = ClientConfig::builder()
    ///     .uri("http://server1:3000")
    ///     .uri("http://server2:3000")
    ///     .uri("http://server3:3000")
    ///     .build();
    /// ```
    pub fn uri(mut self, uri: impl TryInto<Uri, Error = InvalidUri>) -> Self {
        self.config = self.config.and_then(|mut config| {
            config
                .uris
                .push(uri.try_into().map_err(ClientBuilderError::InvalidUri)?);
            Ok(config)
        });
        self
    }

    /// Add some uris
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use poem_grpc::ClientConfig;
    /// let cfg = ClientConfig::builder()
    ///     .uris([
    ///         "http://server1:3000",
    ///         "http://server2:3000",
    ///         "http://server3:3000",
    ///     ])
    ///     .build();
    /// ```
    pub fn uris<I, T>(self, uris: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: TryInto<Uri, Error = InvalidUri>,
    {
        uris.into_iter().fold(self, |acc, uri| acc.uri(uri))
    }

    /// Set `Origin` header for each requests.
    pub fn origin(mut self, origin: impl TryInto<Uri, Error = InvalidUri>) -> Self {
        self.config = self.config.and_then(|mut config| {
            config.origin = Some(
                origin
                    .try_into()
                    .map_err(ClientBuilderError::InvalidOrigin)?,
            );
            Ok(config)
        });
        self
    }

    /// Set `User-Agent` header for each requests.
    pub fn user_agent(
        mut self,
        user_agent: impl TryInto<HeaderValue, Error = InvalidHeaderValue>,
    ) -> Self {
        self.config = self.config.and_then(|mut config| {
            config.user_agent = Some(
                user_agent
                    .try_into()
                    .map_err(ClientBuilderError::InvalidUserAgent)?,
            );
            Ok(config)
        });
        self
    }

    /// Set `TlsConfig` for `HTTPS` uri
    pub fn tls_config(mut self, tls_config: TlsClientConfig) -> Self {
        if let Ok(config) = &mut self.config {
            config.tls_config = Some(tls_config);
        }
        self
    }

    /// Sets the max size of received header frames.
    ///
    /// Default is `16384` bytes.
    pub fn http2_max_header_list_size(mut self, max: u32) -> Self {
        if let Ok(config) = &mut self.config {
            config.max_header_list_size = max;
        }
        self
    }

    /// Consumes this builder and returns the `ClientConfig`
    pub fn build(self) -> Result<ClientConfig, ClientBuilderError> {
        self.config
    }
}

#[doc(hidden)]
#[derive(Clone)]
pub struct GrpcClient {
    ep: Arc<dyn DynEndpoint<Output = HttpResponse> + 'static>,
    send_compressed: Option<CompressionEncoding>,
    accept_compressed: Arc<[CompressionEncoding]>,
}

impl GrpcClient {
    #[inline]
    pub fn new(config: ClientConfig) -> Self {
        Self {
            ep: create_client_endpoint(config),
            send_compressed: None,
            accept_compressed: Arc::new([]),
        }
    }

    pub fn from_endpoint<T>(ep: T) -> Self
    where
        T: IntoEndpoint,
        T::Endpoint: 'static,
        <T::Endpoint as Endpoint>::Output: 'static,
    {
        Self {
            ep: Arc::new(ToDynEndpoint(ep.map_to_response())),
            send_compressed: None,
            accept_compressed: Arc::new([]),
        }
    }

    pub fn set_send_compressed(&mut self, encoding: CompressionEncoding) {
        self.send_compressed = Some(encoding);
    }

    pub fn set_accept_compressed(&mut self, encodings: impl Into<Arc<[CompressionEncoding]>>) {
        self.accept_compressed = encodings.into();
    }

    pub fn with<M>(mut self, middleware: M) -> Self
    where
        M: Middleware<Arc<dyn DynEndpoint<Output = HttpResponse> + 'static>>,
        M::Output: 'static,
    {
        self.ep = Arc::new(ToDynEndpoint(
            middleware.transform(self.ep).map_to_response(),
        ));
        self
    }

    pub async fn unary<T: Codec>(
        &self,
        path: &str,
        mut codec: T,
        request: Request<T::Encode>,
    ) -> Result<Response<T::Decode>, Status> {
        let Request {
            metadata,
            message,
            extensions,
        } = request;
        let mut http_request =
            create_http_request::<T>(path, metadata, extensions, self.send_compressed);
        http_request.set_body(
            create_encode_unary_request_body(codec.encoder(), message, self.send_compressed)
                .await?,
        );

        let mut resp = self
            .ep
            .call(http_request)
            .await
            .map_err(|err| Status::new(Code::Internal).with_message(err))?;

        let incoming_encoding = check_response(&mut resp, &self.accept_compressed).await?;
        let body = resp.take_body();
        let message = decode_single_response_message(
            codec.decoder(),
            resp.headers(),
            body,
            incoming_encoding,
        )
        .await?;
        Ok(Response {
            metadata: Metadata {
                headers: std::mem::take(resp.headers_mut()),
            },
            message,
        })
    }

    pub async fn client_streaming<T: Codec>(
        &self,
        path: &str,
        mut codec: T,
        request: Request<Streaming<T::Encode>>,
    ) -> Result<Response<T::Decode>, Status> {
        let Request {
            metadata,
            message,
            extensions,
        } = request;
        let mut http_request =
            create_http_request::<T>(path, metadata, extensions, self.send_compressed);
        http_request.set_body(create_encode_request_body(
            codec.encoder(),
            message,
            self.send_compressed,
        ));

        let mut resp = self
            .ep
            .call(http_request)
            .await
            .map_err(|err| Status::new(Code::Internal).with_message(err))?;

        let incoming_encoding = check_response(&mut resp, &self.accept_compressed).await?;
        let body = resp.take_body();
        let message = decode_single_response_message(
            codec.decoder(),
            resp.headers(),
            body,
            incoming_encoding,
        )
        .await?;
        Ok(Response {
            metadata: Metadata {
                headers: std::mem::take(resp.headers_mut()),
            },
            message,
        })
    }

    pub async fn server_streaming<T: Codec>(
        &self,
        path: &str,
        mut codec: T,
        request: Request<T::Encode>,
    ) -> Result<Response<Streaming<T::Decode>>, Status> {
        let Request {
            metadata,
            message,
            extensions,
        } = request;
        let mut http_request =
            create_http_request::<T>(path, metadata, extensions, self.send_compressed);
        http_request.set_body(
            create_encode_unary_request_body(codec.encoder(), message, self.send_compressed)
                .await?,
        );

        let mut resp = self
            .ep
            .call(http_request)
            .await
            .map_err(|err| Status::new(Code::Internal).with_message(err))?;

        let incoming_encoding = check_response(&mut resp, &self.accept_compressed).await?;
        let body = resp.take_body();
        let stream =
            create_decode_response_body(codec.decoder(), resp.headers(), body, incoming_encoding)?;

        Ok(Response {
            metadata: Metadata {
                headers: std::mem::take(resp.headers_mut()),
            },
            message: stream,
        })
    }

    pub async fn bidirectional_streaming<T: Codec>(
        &self,
        path: &str,
        mut codec: T,
        request: Request<Streaming<T::Encode>>,
    ) -> Result<Response<Streaming<T::Decode>>, Status> {
        let Request {
            metadata,
            message,
            extensions,
        } = request;
        let mut http_request =
            create_http_request::<T>(path, metadata, extensions, self.send_compressed);
        http_request.set_body(create_encode_request_body(
            codec.encoder(),
            message,
            self.send_compressed,
        ));

        let mut resp = self
            .ep
            .call(http_request)
            .await
            .map_err(|err| Status::new(Code::Internal).with_message(err))?;

        let incoming_encoding = check_response(&mut resp, &self.accept_compressed).await?;
        let body = resp.take_body();
        let stream =
            create_decode_response_body(codec.decoder(), resp.headers(), body, incoming_encoding)?;

        Ok(Response {
            metadata: Metadata {
                headers: std::mem::take(resp.headers_mut()),
            },
            message: stream,
        })
    }
}

/// Rejects a response that is not a gRPC response the client can decode, and
/// returns its message encoding otherwise. A rejected body is discarded first,
/// since dropping it unread resets the stream.
async fn check_response(
    resp: &mut HttpResponse,
    accept_compressed: &[CompressionEncoding],
) -> Result<Option<CompressionEncoding>, Status> {
    let res = if resp.status() != StatusCode::OK {
        Err(Status::new(Code::Internal).with_message(format!(
            "invalid http status code: {}",
            resp.status().as_u16()
        )))
    } else {
        get_incoming_encodings(resp.headers(), accept_compressed)
    };
    if res.is_err() {
        discard_body(resp.take_body()).await;
    }
    res
}

fn create_http_request<T: Codec>(
    path: &str,
    metadata: Metadata,
    extensions: Extensions,
    send_compressed: Option<CompressionEncoding>,
) -> HttpRequest {
    let mut http_request = HttpRequest::builder()
        .uri_str(path)
        .method(Method::POST)
        .version(Version::HTTP_2)
        .finish();
    *http_request.headers_mut() = metadata.headers;
    // Metadata forwarded from an incoming request may carry its length, which
    // doesn't match this body.
    http_request.headers_mut().remove(header::CONTENT_LENGTH);
    *http_request.extensions_mut() = extensions;
    http_request
        .headers_mut()
        .insert("content-type", T::CONTENT_TYPES[0].parse().unwrap());
    http_request
        .headers_mut()
        .insert(header::TE, "trailers".parse().unwrap());
    if let Some(send_compressed) = send_compressed {
        http_request.headers_mut().insert(
            "grpc-encoding",
            HeaderValue::from_str(send_compressed.as_str()).expect("BUG: invalid encoding"),
        );
    }
    http_request
}

#[inline]
fn to_boxed_error(
    err: impl std::error::Error + Send + Sync + 'static,
) -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(err)
}

fn make_uri(base_uri: &Uri, path: &Uri) -> Uri {
    let path = path.path_and_query().unwrap().path();
    let mut parts = base_uri.clone().into_parts();
    match parts.path_and_query {
        Some(path_and_query) => {
            let mut new_path = format!("{}{}", path_and_query.path().trim_end_matches('/'), path);
            if let Some(query) = path_and_query.query() {
                new_path.push('?');
                new_path.push_str(query);
            }
            parts.path_and_query = Some(new_path.parse().unwrap());
        }
        None => {
            parts.path_and_query = Some(path.parse().unwrap());
        }
    }
    Uri::from_parts(parts).unwrap()
}

fn create_client_endpoint(
    config: ClientConfig,
) -> Arc<dyn DynEndpoint<Output = HttpResponse> + 'static> {
    let mut config = config;
    let cli = Client::builder(TokioExecutor::new())
        .http2_only(true)
        .http2_max_header_list_size(config.max_header_list_size)
        .build(HttpsConnector::new(config.tls_config.take()));

    let config = Arc::new(config);

    Arc::new(ToDynEndpoint(poem::endpoint::make(move |request| {
        let config = config.clone();
        let cli = cli.clone();
        async move {
            let mut request: hyper::Request<BoxBody> = request.into();

            if config.uris.is_empty() {
                return Err(poem::Error::from_string(
                    "uris is empty",
                    StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }

            let base_uri = if config.uris.len() == 1 {
                &config.uris[0]
            } else {
                &config.uris[fastrand::usize(0..config.uris.len())]
            };
            *request.uri_mut() = make_uri(base_uri, request.uri());

            if let Some(origin) = &config.origin
                && let Ok(value) = HeaderValue::from_maybe_shared(origin.to_string())
            {
                request.headers_mut().insert(header::ORIGIN, value);
            }

            if let Some(user_agent) = &config.user_agent {
                request
                    .headers_mut()
                    .insert(header::USER_AGENT, user_agent.clone());
            }

            let resp = cli.request(request).await.map_err(to_boxed_error)?;
            let (parts, body) = resp.into_parts();

            Ok::<_, poem::Error>(HttpResponse::from(hyper::Response::from_parts(
                parts,
                body.map_err(IoError::other),
            )))
        }
    })))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use bytes::{BufMut, BytesMut};
    use futures_util::StreamExt;
    use http_body_util::StreamBody;
    use hyper::body::{Body as _, Frame};
    use poem::{
        Body, Server,
        http::HeaderMap,
        listener::{Acceptor, Listener, TcpListener},
    };
    use prost::Message;

    use super::*;
    use crate::codec::ProstCodec;

    #[derive(Clone, PartialEq, Message)]
    struct TestMsg {
        #[prost(int32, tag = 1)]
        value: i32,
    }

    fn data_frame(value: i32) -> Bytes {
        let msg = TestMsg { value }.encode_to_vec();
        let mut buf = BytesMut::new();
        buf.put_u8(0);
        buf.put_u32(msg.len() as u32);
        buf.extend(msg);
        buf.freeze()
    }

    fn grpc_response(data: Vec<Bytes>, status: &'static str, ended: Arc<AtomicBool>) -> Body {
        let mut trailers = HeaderMap::new();
        trailers.insert("grpc-status", HeaderValue::from_static(status));
        trailers.insert("grpc-message", HeaderValue::from_static("from trailers"));
        let frames = futures_util::stream::iter(data)
            .map(|data| Ok::<_, IoError>(Frame::data(data)))
            .chain(futures_util::stream::once(async move {
                ended.store(true, Ordering::SeqCst);
                Ok(Frame::trailers(trailers))
            }));
        BodyExt::boxed(StreamBody::new(frames)).into()
    }

    /// A client whose server responds with `data`, then trailers with
    /// `grpc-status: status`, and a flag set once the trailers are read.
    fn client(data: Vec<Bytes>, status: &'static str) -> (GrpcClient, Arc<AtomicBool>) {
        let ended = Arc::new(AtomicBool::new(false));
        let cli = GrpcClient::from_endpoint(poem::endpoint::make({
            let ended = ended.clone();
            move |_req| {
                let body = grpc_response(data.clone(), status, ended.clone());
                async move {
                    HttpResponse::builder()
                        .content_type("application/grpc")
                        .body(body)
                }
            }
        }));
        (cli, ended)
    }

    async fn unary(cli: &GrpcClient) -> Result<i32, Status> {
        cli.unary(
            "/test.Test/Unary",
            ProstCodec::<TestMsg, TestMsg>::default(),
            Request::new(TestMsg { value: 1 }),
        )
        .await
        .map(|resp| resp.into_inner().value)
    }

    async fn client_streaming(cli: &GrpcClient) -> Result<i32, Status> {
        cli.client_streaming(
            "/test.Test/ClientStreaming",
            ProstCodec::<TestMsg, TestMsg>::default(),
            Request::new_streaming(futures_util::stream::iter(vec![Ok(TestMsg { value: 1 })])),
        )
        .await
        .map(|resp| resp.into_inner().value)
    }

    /// Runs a unary and a client-streaming call against a server responding
    /// with `data` and `status`, and checks that both read the trailers.
    async fn single_response_calls(
        data: Vec<Bytes>,
        status: &'static str,
    ) -> [Result<i32, Status>; 2] {
        let (cli, ended) = client(data.clone(), status);
        let unary = unary(&cli).await;
        assert!(
            ended.load(Ordering::SeqCst),
            "unary didn't read the trailers"
        );
        let (cli, ended) = client(data, status);
        let client_streaming = client_streaming(&cli).await;
        assert!(
            ended.load(Ordering::SeqCst),
            "client streaming didn't read the trailers"
        );
        [unary, client_streaming]
    }

    fn assert_internal(res: Result<i32, Status>, message: &str) {
        let status = res.unwrap_err();
        assert_eq!(status.code(), Code::Internal);
        assert_eq!(status.message(), Some(message));
    }

    #[tokio::test]
    async fn single_response_message() {
        for res in single_response_calls(vec![data_frame(7)], "0").await {
            assert_eq!(res.unwrap(), 7);
        }
    }

    /// The status in the trailers after the message is not ignored.
    #[tokio::test]
    async fn status_from_trailers() {
        for res in single_response_calls(vec![data_frame(7)], "13").await {
            assert_internal(res, "from trailers");
        }
    }

    #[tokio::test]
    async fn too_many_response_messages() {
        let both = [data_frame(7), data_frame(8)].concat().into();
        for data in [vec![data_frame(7), data_frame(8)], vec![both]] {
            for res in single_response_calls(data, "0").await {
                assert_internal(res, "expected only one response message");
            }
        }
    }

    /// A non-OK status in the trailers takes precedence over a local error.
    #[tokio::test]
    async fn trailers_status_over_local_error() {
        let data = vec![data_frame(7), data_frame(8)];
        for res in single_response_calls(data, "13").await {
            assert_internal(res, "from trailers");
        }
    }

    #[tokio::test]
    async fn undecodable_response_message() {
        let data = vec![Bytes::from_static(&[0, 0, 0, 0, 2, 0xff, 0xff])];
        for res in single_response_calls(data, "0").await {
            assert_eq!(res.unwrap_err().code(), Code::Internal);
        }
    }

    /// A response the client can't decode is read to its end before failing.
    #[tokio::test]
    async fn rejected_response_is_read() {
        for (status, encoding) in [(StatusCode::NOT_FOUND, None), (StatusCode::OK, Some("br"))] {
            let ended = Arc::new(AtomicBool::new(false));
            let cli = GrpcClient::from_endpoint(poem::endpoint::make({
                let ended = ended.clone();
                move |_req| {
                    let body = grpc_response(vec![data_frame(7)], "0", ended.clone());
                    async move {
                        let mut resp = HttpResponse::builder()
                            .status(status)
                            .content_type("application/grpc");
                        if let Some(encoding) = encoding {
                            resp = resp.header("grpc-encoding", encoding);
                        }
                        resp.body(body)
                    }
                }
            }));
            assert!(unary(&cli).await.is_err());
            assert!(ended.load(Ordering::SeqCst));
        }
    }

    /// A client whose server responds with `status`, `headers` and `body`.
    fn mock(
        status: StatusCode,
        headers: &'static [(&'static str, &'static str)],
        body: impl Fn() -> Body + Send + Sync + 'static,
    ) -> GrpcClient {
        let body = Arc::new(body);
        GrpcClient::from_endpoint(poem::endpoint::make(move |_req| {
            let body = body();
            async move {
                let mut resp = HttpResponse::builder()
                    .status(status)
                    .content_type("application/grpc");
                for (name, value) in headers {
                    resp = resp.header(*name, *value);
                }
                resp.body(body)
            }
        }))
    }

    /// An endless body of response messages, yielding between them so a test
    /// timeout can fire.
    fn endless_body() -> Body {
        let frames = futures_util::stream::unfold((), |()| async {
            tokio::task::yield_now().await;
            Some((Ok::<_, IoError>(Frame::data(data_frame(7))), ()))
        });
        BodyExt::boxed(StreamBody::new(frames)).into()
    }

    async fn single_response_results(cli: &GrpcClient) -> [Result<i32, Status>; 2] {
        [unary(cli).await, client_streaming(cli).await]
    }

    /// The status of a trailers-only response, i.e. in the headers.
    #[tokio::test]
    async fn trailers_only_response() {
        let cli = mock(
            StatusCode::OK,
            &[("grpc-status", "5"), ("grpc-message", "from headers")],
            Body::empty,
        );
        for res in single_response_results(&cli).await {
            let status = res.unwrap_err();
            assert_eq!(status.code(), Code::NotFound);
            assert_eq!(status.message(), Some("from headers"));
        }

        let cli = mock(StatusCode::OK, &[("grpc-status", "0")], Body::empty);
        for res in single_response_results(&cli).await {
            assert_internal(res, "missing response message");
        }
    }

    #[tokio::test]
    async fn missing_trailers() {
        let cli = mock(StatusCode::OK, &[], || Body::from_bytes(data_frame(7)));
        for res in single_response_results(&cli).await {
            assert_internal(res, "missing trailers");
        }
    }

    #[tokio::test]
    async fn missing_response_message() {
        for res in single_response_calls(vec![], "0").await {
            assert_internal(res, "missing response message");
        }
        let partial = data_frame(7).slice(..6);
        for res in single_response_calls(vec![partial], "0").await {
            assert_eq!(res.unwrap_err().code(), Code::Internal);
        }
    }

    #[tokio::test]
    async fn message_split_across_frames() {
        let msg = data_frame(7);
        let data = vec![msg.slice(..3), msg.slice(3..6), msg.slice(6..)];
        for res in single_response_calls(data, "0").await {
            assert_eq!(res.unwrap(), 7);
        }
    }

    #[tokio::test]
    async fn unsupported_compressed_flag() {
        let data = vec![Bytes::from_static(&[2, 0, 0, 0, 0])];
        for res in single_response_calls(data, "0").await {
            assert_eq!(res.unwrap_err().code(), Code::Unimplemented);
        }
    }

    /// Reading past an error stops after a limit, so an endless response
    /// can't block the call.
    #[tokio::test]
    async fn endless_response_after_error() {
        let timeout = std::time::Duration::from_secs(10);
        let cli = mock(StatusCode::OK, &[], endless_body);
        for res in tokio::time::timeout(timeout, single_response_results(&cli))
            .await
            .unwrap()
        {
            assert_internal(res, "expected only one response message");
        }

        let cli = mock(StatusCode::NOT_FOUND, &[], endless_body);
        for res in tokio::time::timeout(timeout, single_response_results(&cli))
            .await
            .unwrap()
        {
            assert_internal(res, "invalid http status code: 404");
        }
    }

    /// The request of a non-client-streaming call is a single DATA frame with
    /// END_STREAM.
    #[tokio::test]
    async fn request_ends_with_message() {
        let cli =
            GrpcClient::from_endpoint(poem::endpoint::make(|mut req: HttpRequest| async move {
                let mut body: BoxBody = req.take_body().into();
                body.frame().await.unwrap().unwrap();
                let ended = body.is_end_stream() as i32;
                let body = grpc_response(vec![data_frame(ended)], "0", Default::default());
                HttpResponse::builder()
                    .content_type("application/grpc")
                    .body(body)
            }));
        assert_eq!(unary(&cli).await.unwrap(), 1);
        let resp = cli
            .server_streaming(
                "/test.Test/ServerStreaming",
                ProstCodec::<TestMsg, TestMsg>::default(),
                Request::new(TestMsg { value: 1 }),
            )
            .await
            .unwrap();
        let values: Vec<_> = resp
            .into_inner()
            .map(|msg| msg.unwrap().value)
            .collect()
            .await;
        assert_eq!(values, [1]);
    }

    /// The request of a non-client-streaming call is compressed when
    /// configured.
    #[cfg(feature = "gzip")]
    #[tokio::test]
    async fn compressed_request() {
        let mut cli =
            GrpcClient::from_endpoint(poem::endpoint::make(|mut req: HttpRequest| async move {
                let flag = req.take_body().into_bytes().await.unwrap()[0] as i32;
                let body = grpc_response(vec![data_frame(flag)], "0", Default::default());
                HttpResponse::builder()
                    .content_type("application/grpc")
                    .body(body)
            }));
        cli.set_send_compressed(CompressionEncoding::GZIP);
        assert_eq!(unary(&cli).await.unwrap(), 1);
        let resp = cli
            .server_streaming(
                "/test.Test/ServerStreaming",
                ProstCodec::<TestMsg, TestMsg>::default(),
                Request::new(TestMsg { value: 1 }),
            )
            .await
            .unwrap();
        let values: Vec<_> = resp
            .into_inner()
            .map(|msg| msg.unwrap().value)
            .collect()
            .await;
        assert_eq!(values, [1]);
    }

    /// The headers a client sends over a connection: the configured
    /// `User-Agent` and `Origin`, and the body's own `Content-Length` rather
    /// than one forwarded in the metadata.
    #[tokio::test]
    async fn request_headers() {
        let acceptor = TcpListener::bind("127.0.0.1:0")
            .into_acceptor()
            .await
            .unwrap();
        let addr = *acceptor.local_addr()[0].as_socket_addr().unwrap();
        tokio::spawn(
            Server::new_with_acceptor(acceptor).run(poem::endpoint::make(
                |req: HttpRequest| async move {
                    let header = |name| {
                        req.header(name)
                            .map_or("-".to_string(), ToString::to_string)
                    };
                    let value = format!(
                        "{} {} {}",
                        header(header::USER_AGENT),
                        header(header::ORIGIN),
                        header(header::CONTENT_LENGTH),
                    );
                    let mut trailers = HeaderMap::new();
                    trailers.insert("grpc-status", HeaderValue::from_static("0"));
                    let frames = futures_util::stream::iter([
                        Ok::<_, IoError>(Frame::data(data_frame(0))),
                        Ok(Frame::trailers(trailers)),
                    ]);
                    HttpResponse::builder()
                        .content_type("application/grpc")
                        .header("x-headers", value)
                        .body(Body::from(BodyExt::boxed(StreamBody::new(frames))))
                },
            )),
        );

        let cli = GrpcClient::new(
            ClientConfig::builder()
                .uri(format!("http://{addr}"))
                .origin("http://origin.example")
                .user_agent("poem-test/1.0")
                .build()
                .unwrap(),
        );
        let mut req = Request::new(TestMsg { value: 1 });
        req.metadata_mut().insert("content-length", "9999");
        let resp = cli
            .unary(
                "/test.Test/Unary",
                ProstCodec::<TestMsg, TestMsg>::default(),
                req,
            )
            .await
            .unwrap();
        let content_length = data_frame(1).len();
        assert_eq!(
            resp.metadata().get("x-headers"),
            Some(format!("poem-test/1.0 http://origin.example/ {content_length}").as_str())
        );
    }
}
