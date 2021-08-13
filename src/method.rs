use crate::error::{Error, ErrorInvalidMethod, Result};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Method {
    Options = 0,
    Get = 1,
    Post = 2,
    Put = 3,
    Delete = 4,
    Head = 5,
    Trace = 6,
    Connect = 7,
    Patch = 8,
}

pub(crate) const COUNT_METHODS: usize = 9;

impl Method {
    pub(crate) fn from_http_method(method: http::Method) -> Result<Self> {
        Ok(match method {
            http::Method::GET => Method::Get,
            http::Method::POST => Method::Post,
            http::Method::PUT => Method::Put,
            http::Method::DELETE => Method::Delete,
            http::Method::HEAD => Method::Head,
            http::Method::OPTIONS => Method::Options,
            http::Method::CONNECT => Method::Connect,
            http::Method::PATCH => Method::Patch,
            http::Method::TRACE => Method::Trace,
            _ => return Err(Error::internal_server_error(ErrorInvalidMethod)),
        })
    }

    pub(crate) fn into_http_method(self) -> http::Method {
        match self {
            Method::Options => http::Method::OPTIONS,
            Method::Get => http::Method::GET,
            Method::Post => http::Method::POST,
            Method::Put => http::Method::PUT,
            Method::Delete => http::Method::DELETE,
            Method::Head => http::Method::HEAD,
            Method::Trace => http::Method::TRACE,
            Method::Connect => http::Method::CONNECT,
            Method::Patch => http::Method::PATCH,
        }
    }
}
