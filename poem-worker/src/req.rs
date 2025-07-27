use std::net::{IpAddr, SocketAddr};

use http::uri::Scheme;
use http_body_util::combinators::BoxBody;
use poem::{
    Request,
    web::{LocalAddr, RemoteAddr},
};
use worker::{HttpRequest, Result};

pub fn build_poem_req(req: HttpRequest) -> Result<poem::Request> {
    let headers = req.headers();

    let local_addr = if let Some(client_ip) = headers.get("cf-connecting-ip") {
        let client_ip = client_ip
            .to_str()
            .map_err(|e| worker::Error::RustError(format!("{}", e)))?;

        let ip_addr = client_ip
            .parse::<IpAddr>()
            .map_err(|e| worker::Error::RustError(format!("{}", e)))?;

        let addr = SocketAddr::new(ip_addr, 0);

        LocalAddr(poem::Addr::SocketAddr(addr))
    } else {
        LocalAddr::default()
    };

    let remote_addr = RemoteAddr(poem::Addr::Custom("worker", "".into()));
    let scheme = Scheme::HTTPS;

    let (parts, body) = req.into_parts();
    let body = crate::body::WorkerBody(body);
    let boxed_body = BoxBody::new(body);

    let body = poem::Body::from(boxed_body);
    let request_parts = poem::RequestParts::from((parts, local_addr, remote_addr, scheme));

    Ok(Request::from_parts(request_parts, body))
}

pub fn build_worker_resp(resp: poem::Response) -> Result<worker::HttpResponse> {
    let (parts, body) = resp.into_parts();
    let body = crate::body::build_worker_body(body)?;

    let mut builder = http::Response::builder()
        .status(parts.status)
        .version(parts.version)
        .extension(parts.extensions);

    for (key, value) in parts.headers {
        if let Some(key) = key {
            builder = builder.header(key, value);
        }
    }

    let resp = builder.body(body)?;

    Ok(resp)
}
