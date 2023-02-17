use tera::Tera;

use crate::{Endpoint, error::IntoResult, Request, Result, FromRequest, RequestBody, error::InternalServerError, web::Html};

/// Tera Templating Endpoint
pub struct TeraTemplatingEndpoint<E> {
    pub(super) tera: Tera,
    pub(super) inner: E
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for TeraTemplatingEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        req.extensions_mut().insert(self.tera.clone());

        self.inner.call(req).await
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for Tera {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        let tera = req
            .extensions()
            .get::<Tera>()
            .expect("To use the `Tera` extractor, the `TeraTemplating` endpoit is required.")
            .clone();

        Ok(tera)
    }
}

/// Shortcut (or not) for a Tera Templating handler Response
pub type TeraTemplatingResult = tera::Result<String>;

impl IntoResult<Html<String>> for TeraTemplatingResult {
    fn into_result(self) -> Result<Html<String>> {
        self
            .map_err(InternalServerError)
            .map(Html)
    }
}