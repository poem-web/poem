use http::{header, HeaderMap};
use mime::Mime;

use crate::{FromRequest, Request, RequestBody, Result};

/// `Accept` header, defined in [RFC7231](http://tools.ietf.org/html/rfc7231#section-5.3.2)
#[derive(Debug, Clone)]
pub struct Accept(pub Vec<Mime>);

fn parse_accept(headers: &HeaderMap) -> Vec<Mime> {
    let mut items = headers
        .get_all(header::ACCEPT)
        .iter()
        .filter_map(|hval| hval.to_str().ok())
        .flat_map(|s| s.split(',').map(str::trim))
        .filter_map(|v| {
            let (e, q) = match v.split_once(";q=") {
                Some((e, q)) => (e, (q.parse::<f32>().ok()? * 1000.0) as i32),
                None => (v, 1000),
            };
            let mime: Mime = e.parse().ok()?;
            Some((mime, q))
        })
        .collect::<Vec<_>>();
    items.sort_by(|(_, qa), (_, qb)| qb.cmp(qa));
    items.into_iter().map(|(mime, _)| mime).collect()
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for Accept {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        Ok(Self(parse_accept(req.headers())))
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
