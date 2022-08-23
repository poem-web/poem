//! Poem for AWS Lambda.

#![doc(html_favicon_url = "https://raw.githubusercontent.com/poem-web/poem/master/favicon.ico")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/poem-web/poem/master/logo.png")]
#![forbid(unsafe_code)]
#![deny(private_in_public, unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

use std::{io::ErrorKind, ops::Deref, sync::Arc};

pub use lambda_http::lambda_runtime::Error;
use lambda_http::{
    lambda_runtime, service_fn, Body as LambdaBody, Request as LambdaRequest, RequestExt,
};
use poem::{Body, Endpoint, EndpointExt, FromRequest, IntoEndpoint, Request, RequestBody, Result};

/// The Lambda function execution context.
///
/// It implements [`poem::FromRequest`], so it can be used as an extractor.
///
/// # Example
///
/// ```
/// use poem::{handler, Request};
/// use poem_lambda::Context;
///
/// #[handler]
/// fn index(req: &Request, ctx: &Context) {
///     println!("request_id: {}", ctx.request_id);
/// }
/// ```
pub struct Context(pub lambda_runtime::Context);

impl Deref for Context {
    type Target = lambda_runtime::Context;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Starts the AWS Lambda runtime.
///
/// # Example
///
/// ```no_run
/// use poem::handler;
/// use poem_lambda::Error;
///
/// #[handler]
/// fn index() -> &'static str {
///     "hello"
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Error> {
///     poem_lambda::run(index).await
/// }
/// ```
pub async fn run(ep: impl IntoEndpoint) -> Result<(), Error> {
    let ep = Arc::new(ep.map_to_response().into_endpoint());
    lambda_http::run(service_fn(move |req: LambdaRequest| {
        let ctx = req.lambda_context();
        let ep = ep.clone();
        async move {
            let mut req: Request = from_lambda_request(req);
            req.extensions_mut().insert(Context(ctx));

            let resp = ep.get_response(req).await;
            let (parts, body) = resp.into_parts();
            let data = body
                .into_vec()
                .await
                .map_err(|_| std::io::Error::new(ErrorKind::Other, "invalid request"))?;
            let mut lambda_resp = poem::http::Response::new(if data.is_empty() {
                LambdaBody::Empty
            } else {
                match String::from_utf8(data) {
                    Ok(data) => LambdaBody::Text(data),
                    Err(err) => LambdaBody::Binary(err.into_bytes()),
                }
            });
            *lambda_resp.status_mut() = parts.status;
            *lambda_resp.version_mut() = parts.version;
            *lambda_resp.headers_mut() = parts.headers;
            *lambda_resp.extensions_mut() = parts.extensions;

            Ok::<_, Error>(lambda_resp)
        }
    }))
    .await
}

fn from_lambda_request(req: LambdaRequest) -> Request {
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

#[poem::async_trait]
impl<'a> FromRequest<'a> for &'a Context {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        let ctx = match req.extensions().get::<Context>() {
            Some(ctx) => ctx,
            None => panic!("Lambda runtime is required."),
        };
        Ok(ctx)
    }
}
