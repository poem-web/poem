use std::sync::Arc;

use futures_util::TryStreamExt;
use hyper_rustls::HttpsConnectorBuilder;
use poem::{
    http::{
        header, header::InvalidHeaderValue, uri::InvalidUri, Extensions, HeaderValue, StatusCode,
        Uri, Version,
    },
    Endpoint, EndpointExt, IntoEndpoint, Middleware, Request as HttpRequest,
    Response as HttpResponse,
};
use rustls::ClientConfig as TlsClientConfig;

use crate::{
    codec::Codec,
    encoding::{create_decode_response_body, create_encode_request_body},
    Code, Metadata, Request, Response, Status, Streaming,
};

/// A configuration for GRPC client
#[derive(Default)]
pub struct ClientConfig {
    uris: Vec<Uri>,
    origin: Option<Uri>,
    user_agent: Option<HeaderValue>,
    tls_config: Option<TlsClientConfig>,
}

impl ClientConfig {
    /// Create a `ClientConfig` builder
    pub fn builder() -> ClientConfigBuilder {
        ClientConfigBuilder {
            config: Ok(ClientConfig::default()),
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

    /// Consumes this builder and returns the `ClientConfig`
    pub fn build(self) -> Result<ClientConfig, ClientBuilderError> {
        self.config
    }
}

#[doc(hidden)]
#[derive(Clone)]
pub struct GrpcClient {
    ep: Arc<dyn Endpoint<Output = HttpResponse> + 'static>,
}

impl GrpcClient {
    #[inline]
    pub fn new(config: ClientConfig) -> Self {
        Self {
            ep: create_client_endpoint(config),
        }
    }

    pub fn from_endpoint<T>(ep: T) -> Self
    where
        T: IntoEndpoint,
        T::Endpoint: 'static,
        <T::Endpoint as Endpoint>::Output: 'static,
    {
        Self {
            ep: Arc::new(ep.map_to_response()),
        }
    }

    pub fn with<M>(mut self, middleware: M) -> Self
    where
        M: Middleware<Arc<dyn Endpoint<Output = HttpResponse> + 'static>>,
        M::Output: 'static,
    {
        self.ep = Arc::new(middleware.transform(self.ep).map_to_response());
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
        let mut http_request = create_http_request::<T>(path, metadata, extensions);
        http_request.set_body(create_encode_request_body(
            codec.encoder(),
            Streaming::new(futures_util::stream::once(async move { Ok(message) })),
        ));

        let mut resp = self
            .ep
            .call(http_request)
            .await
            .map_err(|err| Status::new(Code::Internal).with_message(err))?;

        if resp.status() != StatusCode::OK {
            return Err(Status::new(Code::Internal).with_message(format!(
                "invalid http status code: {}",
                resp.status().as_u16()
            )));
        }

        let body = resp.take_body();
        let mut stream = create_decode_response_body(codec.decoder(), resp.headers(), body)?;

        let message = stream
            .try_next()
            .await?
            .ok_or_else(|| Status::new(Code::Internal).with_message("missing response message"))?;
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
        let mut http_request = create_http_request::<T>(path, metadata, extensions);
        http_request.set_body(create_encode_request_body(codec.encoder(), message));

        let mut resp = self
            .ep
            .call(http_request)
            .await
            .map_err(|err| Status::new(Code::Internal).with_message(err))?;

        if resp.status() != StatusCode::OK {
            return Err(Status::new(Code::Internal).with_message(format!(
                "invalid http status code: {}",
                resp.status().as_u16()
            )));
        }

        let body = resp.take_body();
        let mut stream = create_decode_response_body(codec.decoder(), resp.headers(), body)?;

        let message = stream
            .try_next()
            .await?
            .ok_or_else(|| Status::new(Code::Internal).with_message("missing response message"))?;
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
        let mut http_request = create_http_request::<T>(path, metadata, extensions);
        http_request.set_body(create_encode_request_body(
            codec.encoder(),
            Streaming::new(futures_util::stream::once(async move { Ok(message) })),
        ));

        let mut resp = self
            .ep
            .call(http_request)
            .await
            .map_err(|err| Status::new(Code::Internal).with_message(err))?;

        if resp.status() != StatusCode::OK {
            return Err(Status::new(Code::Internal).with_message(format!(
                "invalid http status code: {}",
                resp.status().as_u16()
            )));
        }

        let body = resp.take_body();
        let stream = create_decode_response_body(codec.decoder(), resp.headers(), body)?;

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
        let mut http_request = create_http_request::<T>(path, metadata, extensions);
        http_request.set_body(create_encode_request_body(codec.encoder(), message));

        let mut resp = self
            .ep
            .call(http_request)
            .await
            .map_err(|err| Status::new(Code::Internal).with_message(err))?;

        if resp.status() != StatusCode::OK {
            return Err(Status::new(Code::Internal).with_message(format!(
                "invalid http status code: {}",
                resp.status().as_u16()
            )));
        }

        let body = resp.take_body();
        let stream = create_decode_response_body(codec.decoder(), resp.headers(), body)?;

        Ok(Response {
            metadata: Metadata {
                headers: std::mem::take(resp.headers_mut()),
            },
            message: stream,
        })
    }
}

fn create_http_request<T: Codec>(
    path: &str,
    metadata: Metadata,
    extensions: Extensions,
) -> HttpRequest {
    let mut http_request = HttpRequest::builder()
        .uri_str(path)
        .version(Version::HTTP_2)
        .content_type(T::CONTENT_TYPES[0])
        .header(header::TE, "trailers")
        .finish();
    http_request.headers_mut().extend(metadata.headers);
    *http_request.extensions_mut() = extensions;
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
) -> Arc<dyn Endpoint<Output = HttpResponse> + 'static> {
    let mut config = config;
    let cli = match config.tls_config.take() {
        Some(tls_config) => hyper::Client::builder().http2_only(true).build(
            HttpsConnectorBuilder::new()
                .with_tls_config(tls_config)
                .https_or_http()
                .enable_http2()
                .build(),
        ),
        None => hyper::Client::builder().http2_only(true).build(
            HttpsConnectorBuilder::new()
                .with_webpki_roots()
                .https_or_http()
                .enable_http2()
                .build(),
        ),
    };
    let config = Arc::new(config);

    Arc::new(poem::endpoint::make(move |request| {
        let config = config.clone();
        let cli = cli.clone();
        async move {
            let mut request: hyper::Request<hyper::Body> = request.into();

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

            if let Some(origin) = &config.origin {
                if let Ok(value) = HeaderValue::from_maybe_shared(origin.to_string()) {
                    request.headers_mut().insert(header::ORIGIN, value);
                }
            }

            if let Some(user_agent) = &config.user_agent {
                request
                    .headers_mut()
                    .insert(header::ORIGIN, user_agent.clone());
            }

            let resp = cli.request(request).await.map_err(to_boxed_error)?;
            Ok::<_, poem::Error>(HttpResponse::from(resp))
        }
    }))
}
