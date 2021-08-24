use crate::{http::StatusCode, Endpoint, IntoResponse, Request, Response};

/// Endpoint for the [`or`](super::EndpointExt::or) method.
pub struct Or<A, B>(A, B);

impl<A, B> Or<A, B> {
    pub(crate) fn new(a: A, b: B) -> Self {
        Self(a, b)
    }
}

#[async_trait::async_trait]
impl<A, B> Endpoint for Or<A, B>
where
    A: Endpoint,
    B: Endpoint,
{
    type Output = Response;

    async fn call(&self, req: Request) -> Response {
        return if self.0.check(&req) {
            self.0.call(req).await.into_response()
        } else if self.1.check(&req) {
            self.1.call(req).await.into_response()
        } else {
            StatusCode::NOT_FOUND.into()
        };
    }
}
