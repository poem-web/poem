use mime::Mime;
use typed_headers::HeaderMapExt;

use crate::{FromRequest, Request, RequestBody, Result};

/// `Accept` header, defined in [RFC7231](http://tools.ietf.org/html/rfc7231#section-5.3.2)
#[derive(Debug, Clone)]
pub struct Accept(pub Vec<Mime>);

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for Accept {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        let mut items = req
            .headers()
            .typed_get::<typed_headers::Accept>()
            .ok()
            .flatten()
            .map(|accept| accept.0)
            .unwrap_or_default();
        items.sort_by(|a, b| b.quality.cmp(&a.quality));
        Ok(Self(items.into_iter().map(|item| item.item).collect()))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use http::header;

    use super::*;

    #[tokio::test]
    async fn test_accept() {
        let req = Request::builder()
            .header(
                header::ACCEPT,
                "text/html, text/yaml;q=0.5, application/xhtml+xml, application/xml;q=0.9, */*;q=0.1",
            )
            .finish();
        let accept = Accept::from_request_without_body(&req).await.unwrap();
        assert_eq!(
            accept.0,
            &[
                Mime::from_str("text/html").unwrap(),
                Mime::from_str("application/xhtml+xml").unwrap(),
                Mime::from_str("application/xml").unwrap(),
                Mime::from_str("text/yaml").unwrap(),
                Mime::from_str("*/*").unwrap()
            ]
        );
    }
}
