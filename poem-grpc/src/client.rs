use futures_util::TryStreamExt;
use hyper::{header, http::Extensions, StatusCode};
use poem::{
    endpoint::BoxEndpoint, Endpoint, EndpointExt, Middleware, Request as HttpRequest,
    Response as HttpResponse,
};

use crate::{
    codec::Codec,
    streaming::{create_decode_response_stream, create_encode_request_body},
    Code, Metadata, Request, Response, Status, Streaming,
};

#[doc(hidden)]
pub struct GrpcClient {
    base_url: String,
    ep: BoxEndpoint<'static, HttpResponse>,
}

impl GrpcClient {
    #[inline]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ep: create_client_endpoint(),
        }
    }

    pub fn new_with_endpoint<E>(base_url: impl Into<String>, ep: E) -> Self
    where
        E: Endpoint + 'static,
        E::Output: 'static,
    {
        Self {
            base_url: base_url.into(),
            ep: ep.map_to_response().boxed(),
        }
    }

    pub fn with<E>(mut self, middleware: E) -> Self
    where
        E: Middleware<BoxEndpoint<'static, HttpResponse>>,
        E::Output: 'static,
    {
        self.ep = middleware.transform(self.ep).map_to_response().boxed();
        self
    }

    fn make_uri(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
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
        let mut http_request = create_http_request::<T>(self.make_uri(path), metadata, extensions);
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
        let mut stream = create_decode_response_stream(codec.decoder(), resp.headers(), body)?;

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
        let mut http_request = create_http_request::<T>(self.make_uri(path), metadata, extensions);
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
        let mut stream = create_decode_response_stream(codec.decoder(), resp.headers(), body)?;

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
        let mut http_request = create_http_request::<T>(self.make_uri(path), metadata, extensions);
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
        let stream = create_decode_response_stream(codec.decoder(), resp.headers(), body)?;

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
        let mut http_request = create_http_request::<T>(self.make_uri(path), metadata, extensions);
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
        let stream = create_decode_response_stream(codec.decoder(), resp.headers(), body)?;

        Ok(Response {
            metadata: Metadata {
                headers: std::mem::take(resp.headers_mut()),
            },
            message: stream,
        })
    }
}

fn create_http_request<T: Codec>(
    path: String,
    metadata: Metadata,
    extensions: Extensions,
) -> HttpRequest {
    let mut http_request = HttpRequest::builder()
        .uri_str(path)
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

fn create_client_endpoint() -> BoxEndpoint<'static, HttpResponse> {
    let cli = hyper::Client::builder().http2_only(true).build_http();
    poem::endpoint::make(move |request| {
        let cli = cli.clone();
        async move {
            let request: hyper::Request<hyper::Body> = request.into();
            let resp = cli.request(request).await.map_err(to_boxed_error)?;
            Ok::<_, poem::Error>(HttpResponse::from(resp))
        }
    })
    .boxed()
}
