use std::sync::Arc;

use http::StatusCode;
use poem::{FromRequest, Request, RequestBody};

#[derive(Clone)]
pub struct Context(Arc<worker::Context>);

impl Context {
    pub fn new(ctx: worker::Context) -> Self {
        Self(Arc::new(ctx))
    }

    pub fn wait_until<F>(&self, future: F)
    where
        F: Future<Output = ()> + 'static,
    {
        self.0.wait_until(future);
    }

    pub fn pass_through_on_exception(&self) {
        self.0.pass_through_on_exception();
    }
}

impl<'a> FromRequest<'a> for Context {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> poem::Result<Self> {
        let ctx = req.data::<Context>().ok_or_else(|| {
            poem::Error::from_string("failed to get incoming context", StatusCode::BAD_REQUEST)
        })?;

        Ok(ctx.clone())
    }
}
