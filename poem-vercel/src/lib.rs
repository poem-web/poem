mod types;

use std::ops::Deref;

use lambda_http::lambda_runtime;
use poem::IntoEndpoint;

use crate::{
    lambda_runtime::Error,
    types::{VercelEvent, VercelResponse},
};

/// The Lambda function execution context.
///
/// It implements [`poem::FromRequest`], so it can be used as an extractor.
///
/// # Example
///
/// ```
/// use poem::{handler, Request};
/// use poem_vercel::Context;
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

pub async fn run(ep: impl IntoEndpoint) -> Result<(), Error> {
    let ep = ep.into_endpoint();
    lambda_runtime::run(lambda_runtime::handler_fn(
        move |event: VercelEvent, ctx: lambda_runtime::Context| async move {
            Ok::<_, Error>(VercelResponse {
                status_code: 0,
                headers: Default::default(),
                body: None,
                encoding: None,
            })
        },
    ))
    .await
}
