use std::ops::{Deref, DerefMut};

use serde::de::DeserializeOwned;

use crate::{error::ParseQueryError, FromRequest, Request, RequestBody, Result};

/// An extractor that can deserialize some type from query string.
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Query<T>(pub T);

impl<T> Deref for Query<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Query<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[async_trait::async_trait]
impl<'a, T: DeserializeOwned> FromRequest<'a> for Query<T> {
    type Error = ParseQueryError;

    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self, Self::Error> {
        Ok(serde_urlencoded::from_str(req.uri().query().unwrap_or_default()).map(Self)?)
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use crate::{handler, http::Uri, Endpoint};

    #[tokio::test]
    async fn test_query_extractor() {
        #[derive(Deserialize)]
        struct CreateResource {
            name: String,
            value: i32,
        }

        #[handler(internal)]
        async fn index(query: Query<CreateResource>) {
            assert_eq!(query.name, "abc");
            assert_eq!(query.value, 100);
        }

        index
            .call(
                Request::builder()
                    .uri(Uri::from_static("/?name=abc&value=100"))
                    .finish(),
            )
            .await;
    }
}
