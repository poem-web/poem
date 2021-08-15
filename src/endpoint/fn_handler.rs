use std::{future::Future, marker::PhantomData};

use super::Endpoint;
use crate::{
    error::Result,
    request::Request,
    response::Response,
    web::{FromRequest, IntoResponse},
};

/// Represents a handler that can handle requests.
#[async_trait::async_trait]
pub trait FnHandler<In>: Send + Sync {
    /// Call the handler with the given request.
    async fn call(&self, req: Request) -> Result<Response>;
}

macro_rules! impl_fn_handler {
    () => {};

    ($head: ident, $($tail:ident),* $(,)?) => {
        #[async_trait::async_trait]
        impl<F, Fut, Res, $head, $($tail,)*> FnHandler<($head, $($tail,)*)> for F
        where
            F: Fn($head, $($tail,)*) -> Fut + Send + Sync,
            Fut: Future<Output = Res> + Send,
            Res: IntoResponse,
            $head: FromRequest + Send,
            $($tail: FromRequest + Send,)* {
            #[allow(non_snake_case)]
            async fn call(&self, mut req: Request) -> Result<Response> {
                let $head = $head::from_request(&mut req).await?;
                $(
                let $tail = $tail::from_request(&mut req).await?;
                )*
                self($head, $($tail,)*).await.into_response()
            }
        }

        impl_fn_handler!($($tail,)*);
    };
}

impl_fn_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);

#[async_trait::async_trait]
impl<F, Fut, Res> FnHandler<()> for F
where
    F: Fn() -> Fut + Send + Sync,
    Fut: Future<Output = Res> + Send,
    Res: IntoResponse,
{
    async fn call(&self, _req: Request) -> Result<Response> {
        self().await.into_response()
    }
}

#[async_trait::async_trait]
impl<F, Fut, Res> FnHandler<Request> for F
where
    F: Fn(Request) -> Fut + Send + Sync,
    Fut: Future<Output = Res> + Send,
    Res: IntoResponse,
{
    async fn call(&self, req: Request) -> Result<Response> {
        self(req).await.into_response()
    }
}

pub(crate) struct FnHandlerWrapper<F, In> {
    f: F,
    _mark: PhantomData<In>,
}

impl<F, In> FnHandlerWrapper<F, In>
where
    F: FnHandler<In>,
{
    pub(crate) fn new(f: F) -> Self {
        Self {
            f,
            _mark: PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<In, F> Endpoint for FnHandlerWrapper<F, In>
where
    In: Send + Sync + 'static,
    F: FnHandler<In> + 'static,
{
    async fn call(&self, req: Request) -> Result<Response> {
        self.f.call(req).await
    }
}
