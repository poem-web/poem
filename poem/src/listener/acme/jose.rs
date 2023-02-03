use std::io::{Error as IoError, ErrorKind, Result as IoResult};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use http::{Method, Uri};
use hyper::{client::HttpConnector, Client};
use hyper_rustls::HttpsConnector;
use ring::digest::{digest, Digest, SHA256};
use serde::{de::DeserializeOwned, Serialize};

use crate::{listener::acme::keypair::KeyPair, Request, Response};

#[derive(Serialize)]
struct Protected<'a> {
    alg: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    jwk: Option<Jwk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kid: Option<&'a str>,
    nonce: &'a str,
    url: &'a str,
}

impl<'a> Protected<'a> {
    fn base64(
        jwk: Option<Jwk>,
        kid: Option<&'a str>,
        nonce: &'a str,
        url: &'a str,
    ) -> IoResult<String> {
        let protected = Self {
            alg: "ES256",
            jwk,
            kid,
            nonce,
            url,
        };
        let protected = serde_json::to_vec(&protected).map_err(|err| {
            IoError::new(ErrorKind::Other, format!("failed to encode jwt: {err}"))
        })?;
        Ok(URL_SAFE_NO_PAD.encode(protected))
    }
}

#[derive(Serialize)]
struct Jwk {
    alg: &'static str,
    crv: &'static str,
    kty: &'static str,
    #[serde(rename = "use")]
    u: &'static str,
    x: String,
    y: String,
}

impl Jwk {
    fn new(key: &KeyPair) -> Self {
        let (x, y) = key.public_key()[1..].split_at(32);
        Self {
            alg: "ES256",
            crv: "P-256",
            kty: "EC",
            u: "sig",
            x: URL_SAFE_NO_PAD.encode(x),
            y: URL_SAFE_NO_PAD.encode(y),
        }
    }

    fn thumb_sha256_base64(&self) -> IoResult<String> {
        #[derive(Serialize)]
        struct JwkThumb<'a> {
            crv: &'a str,
            kty: &'a str,
            x: &'a str,
            y: &'a str,
        }

        let jwk_thumb = JwkThumb {
            crv: self.crv,
            kty: self.kty,
            x: &self.x,
            y: &self.y,
        };
        let json = serde_json::to_vec(&jwk_thumb).map_err(|err| {
            IoError::new(ErrorKind::Other, format!("failed to encode jwt: {err}"))
        })?;
        let hash = sha256(json);
        Ok(URL_SAFE_NO_PAD.encode(hash))
    }
}

fn sha256(data: impl AsRef<[u8]>) -> Digest {
    digest(&SHA256, data.as_ref())
}

#[derive(Serialize)]
struct Body {
    protected: String,
    payload: String,
    signature: String,
}

pub(crate) async fn request(
    cli: &Client<HttpsConnector<HttpConnector>>,
    key_pair: &KeyPair,
    kid: Option<&str>,
    nonce: &str,
    uri: &Uri,
    payload: Option<impl Serialize>,
) -> IoResult<Response> {
    let jwk = match kid {
        None => Some(Jwk::new(key_pair)),
        Some(_) => None,
    };
    let protected = Protected::base64(jwk, kid, nonce, &uri.to_string())?;
    let payload = match payload {
        Some(payload) => serde_json::to_vec(&payload).map_err(|err| {
            IoError::new(ErrorKind::Other, format!("failed to encode payload: {err}"))
        })?,
        None => Vec::new(),
    };
    let payload = URL_SAFE_NO_PAD.encode(payload);
    let combined = format!("{}.{}", &protected, &payload);
    let signature = URL_SAFE_NO_PAD.encode(key_pair.sign(combined.as_bytes())?);
    let body = serde_json::to_vec(&Body {
        protected,
        payload,
        signature,
    })
    .unwrap();
    let req = Request::builder()
        .method(Method::POST)
        .uri(uri.clone())
        .content_type("application/jose+json")
        .body(body);

    tracing::debug!(uri = %uri, "http request");

    let resp = cli.request(req.into()).await.map_err(|err| {
        IoError::new(
            ErrorKind::Other,
            format!("failed to send http request: {err}"),
        )
    })?;
    if !resp.status().is_success() {
        return Err(IoError::new(
            ErrorKind::Other,
            format!("unexpected status code: status = {}", resp.status()),
        ));
    }
    Ok(resp.into())
}

pub(crate) async fn request_json<T, R>(
    cli: &Client<HttpsConnector<HttpConnector>>,
    key_pair: &KeyPair,
    kid: Option<&str>,
    nonce: &str,
    uri: &Uri,
    payload: Option<T>,
) -> IoResult<R>
where
    T: Serialize,
    R: DeserializeOwned,
{
    let resp = request(cli, key_pair, kid, nonce, uri, payload).await?;

    let data = resp
        .into_body()
        .into_string()
        .await
        .map_err(|_| IoError::new(ErrorKind::Other, "failed to read response"))?;
    serde_json::from_str(&data)
        .map_err(|err| IoError::new(ErrorKind::Other, format!("bad response: {err}")))
}

pub(crate) fn key_authorization(key: &KeyPair, token: &str) -> IoResult<String> {
    let jwk = Jwk::new(key);
    let key_authorization = format!("{}.{}", token, jwk.thumb_sha256_base64()?);
    Ok(key_authorization)
}

pub(crate) fn key_authorization_sha256(key: &KeyPair, token: &str) -> IoResult<impl AsRef<[u8]>> {
    Ok(sha256(key_authorization(key, token)?.as_bytes()))
}
