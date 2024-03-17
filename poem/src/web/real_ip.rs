use std::net::IpAddr;

use rfc7239::{NodeIdentifier, NodeName};

use crate::{Addr, FromRequest, Request, RequestBody, Result};

/// An extractor that can extracts the real ip from request headers
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct RealIp(pub Option<IpAddr>);

impl<'a> FromRequest<'a> for RealIp {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        if let Some(real_ip) = req
            .headers()
            .get("x-real-ip")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<IpAddr>().ok())
        {
            return Ok(RealIp(Some(real_ip)));
        }

        if let Some(forwarded) = req
            .headers()
            .get("forwarded")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| rfc7239::parse(value).collect::<Result<Vec<_>, _>>().ok())
        {
            if let Some(real_ip) = forwarded
                .into_iter()
                .find_map(|item| match item.forwarded_for {
                    Some(NodeIdentifier {
                        name: NodeName::Ip(ip_addr),
                        ..
                    }) => Some(ip_addr),
                    _ => None,
                })
            {
                return Ok(RealIp(Some(real_ip)));
            }
        }

        if let Some(real_ip) = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| {
                value
                    .split(',')
                    .map(|value| value.trim())
                    .find_map(|value| value.parse::<IpAddr>().ok())
            })
        {
            return Ok(RealIp(Some(real_ip)));
        }

        match req.remote_addr().0 {
            Addr::SocketAddr(addr) => Ok(RealIp(Some(addr.ip()))),
            _ => Ok(RealIp(None)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_request(header: &str, value: &str) -> Request {
        Request::builder().header(header, value).finish()
    }

    #[tokio::test]
    async fn test_realip_extractor() {
        assert_eq!(
            RealIp::from_request_without_body(&create_request("x-real-ip", "203.0.113.195"))
                .await
                .unwrap(),
            RealIp(Some("203.0.113.195".parse().unwrap()))
        );

        assert_eq!(
            RealIp::from_request_without_body(&create_request(
                "x-forwarded-for",
                "203.0.113.195, 70.41.3.18, 150.172.238.178"
            ))
            .await
            .unwrap(),
            RealIp(Some("203.0.113.195".parse().unwrap()))
        );

        assert_eq!(
            RealIp::from_request_without_body(&create_request(
                "forwarded",
                "for=192.0.2.43, for=198.51.100.17"
            ))
            .await
            .unwrap(),
            RealIp(Some("192.0.2.43".parse().unwrap()))
        );
    }
}
