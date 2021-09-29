use std::{convert::Infallible, sync::Arc};

pub use lambda_http::lambda_runtime::Error;
use lambda_http::{handler, lambda_runtime::Context, Body as LambdaBody, Request as LambdaRequest};

use crate::{Body, Endpoint, EndpointExt, FromRequest, IntoEndpoint, Request, RequestBody};

struct InternalData<T>(T);

/// Starts the Lambda Rust runtime and begins polling for events on the [Lambda Runtime APIs](https://docs.aws.amazon.com/lambda/latest/dg/runtimes-api.html).
pub async fn run(ep: impl IntoEndpoint) -> Result<(), Error> {
    let ep = Arc::new(ep.map_to_response().into_endpoint());
    lambda_http::lambda_runtime::run(handler(move |req: LambdaRequest, ctx: Context| {
        let ep = ep.clone();
        async move {
            let mut req: Request = req.into();
            req.extensions_mut().insert(InternalData(ctx));

            let resp = ep.call(req).await;

            let (parts, body) = resp.into_parts();
            let data = body.into_vec().await.map_err(Box::new)?;
            let mut lambda_resp = http::Response::new(if data.is_empty() {
                LambdaBody::Empty
            } else {
                LambdaBody::Binary(data)
            });
            *lambda_resp.status_mut() = parts.status;
            *lambda_resp.version_mut() = parts.version;
            *lambda_resp.headers_mut() = parts.headers;
            *lambda_resp.extensions_mut() = parts.extensions;

            Ok::<_, Error>(lambda_http::IntoResponse::into_response(lambda_resp))
        }
    }))
    .await
}

impl From<LambdaRequest> for Request {
    fn from(req: LambdaRequest) -> Self {
        let (parts, lambda_body) = req.into_parts();
        let body = match lambda_body {
            LambdaBody::Empty => Body::empty(),
            LambdaBody::Text(data) => Body::from_string(data),
            LambdaBody::Binary(data) => Body::from_vec(data),
        };
        let mut req = Request::builder()
            .method(parts.method)
            .uri(parts.uri)
            .version(parts.version)
            .body(body);
        *req.headers_mut() = parts.headers;
        *req.extensions_mut() = parts.extensions;
        req
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for &'a Context {
    type Error = Infallible;

    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self, Self::Error> {
        let ctx = match req.extensions().get::<InternalData<Context>>() {
            Some(ctx) => &ctx.0,
            None => panic!("Lambda runtime is required."),
        };
        Ok(ctx)
    }
}
