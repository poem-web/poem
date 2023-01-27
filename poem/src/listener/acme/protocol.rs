use std::{
    fmt::{self, Display, Formatter},
    io::{Error as IoError, ErrorKind, Result as IoResult},
};

use serde::{Deserialize, Serialize};

use crate::listener::acme::serde::SerdeUri;

/// HTTP-01 challenge
const CHALLENGE_TYPE_HTTP_01: &str = "http-01";

/// TLS-ALPN-01 challenge
const CHALLENGE_TYPE_TLS_ALPN_01: &str = "tls-alpn-01";

/// Challenge type
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ChallengeType {
    /// HTTP-01 challenge
    ///
    /// Reference: <https://letsencrypt.org/docs/challenge-types/#http-01-challenge>
    Http01,
    /// TLS-ALPN-01
    ///
    /// Reference: <https://letsencrypt.org/docs/challenge-types/#tls-alpn-01>
    TlsAlpn01,
}

impl Display for ChallengeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ChallengeType::Http01 => f.write_str(CHALLENGE_TYPE_HTTP_01),
            ChallengeType::TlsAlpn01 => f.write_str(CHALLENGE_TYPE_TLS_ALPN_01),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Directory {
    pub(crate) new_nonce: SerdeUri,
    pub(crate) new_account: SerdeUri,
    pub(crate) new_order: SerdeUri,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NewAccountRequest {
    pub(crate) only_return_existing: bool,
    pub(crate) terms_of_service_agreed: bool,
    pub(crate) contacts: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Identifier {
    #[serde(rename = "type")]
    pub(crate) ty: String,
    pub(crate) value: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NewOrderRequest {
    pub(crate) identifiers: Vec<Identifier>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Problem {
    pub(crate) detail: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NewOrderResponse {
    pub(crate) status: String,
    pub(crate) authorizations: Vec<SerdeUri>,
    pub(crate) error: Option<Problem>,
    pub(crate) finalize: SerdeUri,
    pub(crate) certificate: Option<SerdeUri>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Challenge {
    #[serde(rename = "type")]
    pub(crate) ty: String,
    pub(crate) url: SerdeUri,
    pub(crate) token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FetchAuthorizationResponse {
    pub(crate) identifier: Identifier,
    pub(crate) status: String,
    pub(crate) challenges: Vec<Challenge>,
    pub(crate) error: Option<Problem>,
}

impl FetchAuthorizationResponse {
    pub(crate) fn find_challenge(&self, ty: ChallengeType) -> IoResult<&Challenge> {
        self.challenges
            .iter()
            .find(|c| c.ty == ty.to_string())
            .ok_or_else(|| {
                IoError::new(ErrorKind::Other, format!("unable to find `{ty}` challenge"))
            })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CsrRequest {
    pub(crate) csr: String,
}
