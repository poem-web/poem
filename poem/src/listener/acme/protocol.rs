use std::io::{Error as IoError, ErrorKind, Result as IoResult};

use serde::{Deserialize, Serialize};

use crate::listener::acme::serde::SerdeUri;

pub(crate) const CHALLENGE_TYPE_TLS_ALPN_01: &str = "tls-alpn-01";

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
    pub(crate) contact: Vec<String>,
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
    pub(crate) fn find_challenge(&self, ty: &str) -> IoResult<&Challenge> {
        self.challenges.iter().find(|c| c.ty == ty).ok_or_else(|| {
            IoError::new(
                ErrorKind::Other,
                format!("unable to find `{}` challenge", CHALLENGE_TYPE_TLS_ALPN_01),
            )
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CsrRequest {
    pub(crate) csr: String,
}
