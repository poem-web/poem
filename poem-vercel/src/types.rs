use std::str::FromStr;

use lambda_runtime::Error;
use poem::{
    http::{HeaderMap, Method, Uri},
    Body, Request, Response,
};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

#[derive(Deserialize, Debug)]
pub(crate) struct VercelEvent {
    #[serde(rename = "Action")]
    pub(crate) action: String,
    #[serde(deserialize_with = "deserialize_event_body")]
    pub(crate) body: VercelRequest,
}

fn deserialize_event_body<'de, D>(deserializer: D) -> Result<VercelRequest, D::Error>
where
    D: Deserializer<'de>,
{
    serde_json::from_str(&String::deserialize(deserializer)?).map_err(D::Error::custom)
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VercelRequest {
    pub(crate) host: String,
    pub(crate) path: String,
    #[serde(with = "http_serde::method")]
    pub(crate) method: Method,
    #[serde(with = "http_serde::header_map")]
    pub(crate) headers: HeaderMap,
    pub(crate) body: Option<String>,
    pub(crate) encoding: Option<String>,
}

impl TryFrom<VercelRequest> for Request {
    type Error = Error;

    fn try_from(
        VercelRequest {
            host,
            path,
            method,
            headers,
            body,
            encoding,
        }: VercelRequest,
    ) -> Result<Self, Self::Error> {
        let body = match (body, encoding.as_deref()) {
            (Some(data), Some("base64")) => Body::from(base64::decode(data)?),
            (Some(data), _) => Body::from(data),
            (None, _) => Body::empty(),
        };

        let mut request = Request::builder()
            .method(method)
            .uri(Uri::from_str(&format!("https://{}{}", host, path))?)
            .body(body);
        *request.headers_mut() = headers;

        Ok(request)
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VercelResponse {
    pub(crate) status_code: u16,
    #[serde(
        skip_serializing_if = "HeaderMap::is_empty",
        with = "http_serde::header_map"
    )]
    pub(crate) headers: HeaderMap,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) encoding: Option<&'static str>,
}

pub(crate) async fn to_vercel_response(resp: Response) -> Result<VercelResponse, Error> {
    let (parts, body) = resp.into_parts();
    let data = body.into_vec().await?;
    let (body, encoding) = if data.is_empty() {
        (None, None)
    } else {
        match String::from_utf8(data) {
            Ok(data) => (Some(data), None),
            Err(err) => (Some(base64::encode(err.into_bytes())), Some("base64")),
        }
    };

    Ok(VercelResponse {
        status_code: parts.status.as_u16(),
        headers: parts.headers,
        body,
        encoding,
    })
}
