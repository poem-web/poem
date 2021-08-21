use std::convert::TryInto;

use crate::{
    http::{header::HeaderName, HeaderValue},
    Endpoint, Middleware, Request, Response, Result,
};

enum Action {
    Override(HeaderName, HeaderValue),
    Append(HeaderName, HeaderValue),
}

/// Middleware for override/append headers to response.
#[derive(Default)]
pub struct SetHeader {
    actions: Vec<Action>,
}

impl SetHeader {
    /// Create new [SetHeader] middleware.
    #[must_use]
    pub fn new() -> Self {
        Default::default()
    }

    /// Inserts a header to response.
    ///
    /// If a previous value exists for the same header, it is
    /// removed and replaced with the new header value.
    #[must_use]
    pub fn overriding<K, V>(mut self, key: K, value: V) -> Self
    where
        K: TryInto<HeaderName>,
        V: TryInto<HeaderValue>,
    {
        let key = key.try_into();
        let value = value.try_into();
        if let (Ok(key), Ok(value)) = (key, value) {
            self.actions.push(Action::Override(key, value));
        }
        self
    }

    /// Appends a header to response.
    ///
    /// If previous values exist, the header will have multiple values.
    #[must_use]
    pub fn appending<K, V>(mut self, key: K, value: V) -> Self
    where
        K: TryInto<HeaderName>,
        V: TryInto<HeaderValue>,
    {
        let key = key.try_into();
        let value = value.try_into();
        if let (Ok(key), Ok(value)) = (key, value) {
            self.actions.push(Action::Append(key, value));
        }
        self
    }
}

impl<E: Endpoint> Middleware<E> for SetHeader {
    type Output = SetHeaderImpl<E>;

    fn transform(self, ep: E) -> Self::Output {
        SetHeaderImpl {
            inner: ep,
            actions: self.actions,
        }
    }
}

#[doc(hidden)]
pub struct SetHeaderImpl<E> {
    inner: E,
    actions: Vec<Action>,
}

#[async_trait::async_trait]
impl<E> Endpoint for SetHeaderImpl<E>
where
    E: Endpoint,
{
    async fn call(&self, req: Request) -> Result<Response> {
        let mut resp = self.inner.call(req).await?;
        let headers = resp.headers_mut();

        for action in &self.actions {
            match action {
                Action::Override(name, value) => {
                    headers.insert(name.clone(), value.clone());
                }
                Action::Append(name, value) => {
                    headers.append(name.clone(), value.clone());
                }
            }
        }

        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{get, EndpointExt};

    #[tokio::test]
    async fn test_set_header() {
        #[get(internal)]
        fn index() {}

        let resp = index
            .with(
                SetHeader::new()
                    .overriding("custom-a", "a")
                    .overriding("custom-a", "b")
                    .appending("custom-b", "a")
                    .appending("custom-b", "b"),
            )
            .call(Request::default())
            .await
            .unwrap();

        assert_eq!(
            resp.headers()
                .get_all("custom-a")
                .into_iter()
                .filter_map(|value| value.to_str().ok())
                .collect::<Vec<_>>(),
            vec!["b"]
        );

        assert_eq!(
            resp.headers()
                .get_all("custom-b")
                .into_iter()
                .filter_map(|value| value.to_str().ok())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }
}
