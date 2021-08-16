use crate::{
    error::{ErrorCookieIllegal, ErrorNoCookie},
    http::header,
    Body, FromRequest, Request, Result,
};

/// Representation of an HTTP cookie.
pub type Cookie = cookie::Cookie<'static>;

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for Cookie {
    async fn from_request(req: &'a Request, _body: &mut Option<Body>) -> Result<Self> {
        let value = req.headers().get(header::COOKIE).ok_or(ErrorNoCookie)?;
        let value = value.to_str().map_err(|_| ErrorCookieIllegal)?;
        let cookie = cookie::Cookie::parse(value.to_string()).map_err(|_| ErrorCookieIllegal)?;
        Ok(cookie)
    }
}

#[async_trait::async_trait]
impl<'a> FromRequest<'a> for &cookie::CookieJar {
    async fn from_request(_req: &'a Request, _body: &mut Option<Body>) -> Result<Self> {
        todo!()
    }
}
