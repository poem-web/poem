//! Commonly used middleware.

mod add_data;
mod cors;
mod set_header;
#[cfg(feature = "tracing")]
mod tracing;

pub use add_data::AddData;
pub use cors::Cors;
pub use set_header::SetHeader;

#[cfg(feature = "tracing")]
pub use self::tracing::Tracing;
use crate::endpoint::Endpoint;

/// Represents a middleware trait.
pub trait Middleware<E: Endpoint> {
    /// New endpoint type.
    ///
    /// If you don't know what type to use, then you can use [`Box<dyn
    /// Endpoint>`], which will bring some performance loss, but it is
    /// insignificant.
    type Output: Endpoint;

    /// Transform the input [`Endpoint`] to another one.
    fn transform(self, ep: E) -> Self::Output;
}

/// A middleware implemented by a closure.
pub struct FnMiddleware<T>(T);

impl<T, E, E2> Middleware<E> for FnMiddleware<T>
where
    T: Fn(E) -> E2,
    E: Endpoint,
    E2: Endpoint,
{
    type Output = E2;

    fn transform(self, ep: E) -> Self::Output {
        (self.0)(ep)
    }
}

/// Make middleware with a closure.
pub fn make<T>(f: T) -> FnMiddleware<T> {
    FnMiddleware(f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        handler,
        http::{header::HeaderName, HeaderValue},
        EndpointExt, IntoResponse, Request, Response,
    };

    #[tokio::test]
    async fn test_make() {
        #[handler(internal)]
        fn index() -> &'static str {
            "abc"
        }

        struct AddHeader<E> {
            ep: E,
            header: HeaderName,
            value: HeaderValue,
        }

        #[async_trait::async_trait]
        impl<E: Endpoint> Endpoint for AddHeader<E> {
            type Output = Response;

            async fn call(&self, req: Request) -> Self::Output {
                let mut resp = self.ep.call(req).await.into_response();
                resp.headers_mut()
                    .insert(self.header.clone(), self.value.clone());
                resp
            }
        }

        let ep = index.with(make(|ep| AddHeader {
            ep,
            header: HeaderName::from_static("hello"),
            value: HeaderValue::from_static("world"),
        }));
        let mut resp = ep.call(Request::default()).await;
        assert_eq!(
            resp.headers()
                .get(HeaderName::from_static("hello"))
                .cloned(),
            Some(HeaderValue::from_static("world"))
        );
        assert_eq!(resp.take_body().into_string().await.unwrap(), "abc");
    }
}
