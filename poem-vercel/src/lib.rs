mod types;

use std::{ops::Deref, sync::Arc};

use lambda_runtime::Error;
use poem::{Endpoint, EndpointExt, IntoEndpoint, Request};

use crate::types::{to_vercel_response, VercelEvent};

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

/// Starts the Vercel lambda runtime.
///
/// # Example
///
/// ```no_run
/// use poem::handler;
/// use poem_vercel::Error;
///
/// #[handler]
/// fn index() -> &'static str {
///     "hello"
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Error> {
///     poem_vercel::run(index).await
/// }
/// ```
pub async fn run(ep: impl IntoEndpoint) -> Result<(), Error> {
    let ep = Arc::new(ep.map_to_response().into_endpoint());
    lambda_runtime::run(lambda_runtime::handler_fn(
        move |event: VercelEvent, ctx: lambda_runtime::Context| {
            let ep = ep.clone();
            async move {
                let mut req: Request = event.body.try_into()?;
                req.extensions_mut().insert(Context(ctx));
                to_vercel_response(ep.get_response(req).await).await
            }
        },
    ))
    .await
}
