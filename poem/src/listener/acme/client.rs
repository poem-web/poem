use std::{
    io::{Error as IoError, Result as IoResult},
    sync::Arc,
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::Client;

use crate::listener::acme::{
    ChallengeType, jose,
    keypair::KeyPair,
    protocol::{
        CsrRequest, Directory, FetchAuthorizationResponse, Identifier, NewAccountRequest,
        NewOrderRequest, NewOrderResponse,
    },
};

/// A client for ACME-supporting TLS certificate services.
pub struct AcmeClient {
    client: Client,
    directory: Directory,
    pub(crate) key_pair: Arc<KeyPair>,
    contacts: Vec<String>,
    kid: Option<String>,
}

impl AcmeClient {
    /// Create a new client. `directory_url` is the url for the ACME provider.
    /// `contacts` is a list of URLS (ex: `mailto:`) the ACME service can
    /// use to reach you if there's issues with your certificates.
    pub async fn try_new(directory_url: &str, contacts: Vec<String>) -> IoResult<Self> {
        let client = Client::new();
        let directory = get_directory(&client, directory_url).await?;
        Ok(Self {
            client,
            directory,
            key_pair: Arc::new(KeyPair::generate()?),
            contacts,
            kid: None,
        })
    }

    pub(crate) async fn new_order<T: AsRef<str>>(
        &mut self,
        domains: &[T],
    ) -> IoResult<NewOrderResponse> {
        let kid = match &self.kid {
            Some(kid) => kid,
            None => {
                // create account
                let kid = create_acme_account(
                    &self.client,
                    &self.directory,
                    &self.key_pair,
                    self.contacts.clone(),
                )
                .await?;
                self.kid = Some(kid);
                self.kid.as_ref().unwrap()
            }
        };

        tracing::debug!(kid = kid.as_str(), "new order request");

        let nonce = get_nonce(&self.client, &self.directory).await?;
        let resp: NewOrderResponse = jose::request_json(
            &self.client,
            &self.key_pair,
            Some(kid),
            &nonce,
            &self.directory.new_order,
            Some(NewOrderRequest {
                identifiers: domains
                    .iter()
                    .map(|domain| Identifier {
                        ty: "dns".to_string(),
                        value: domain.as_ref().to_string(),
                    })
                    .collect(),
            }),
        )
        .await?;

        tracing::debug!(status = resp.status.as_str(), "order created");
        Ok(resp)
    }

    pub(crate) async fn fetch_authorization(
        &self,
        auth_url: &str,
    ) -> IoResult<FetchAuthorizationResponse> {
        tracing::debug!(auth_uri = %auth_url, "fetch authorization");

        let nonce = get_nonce(&self.client, &self.directory).await?;
        let resp: FetchAuthorizationResponse = jose::request_json(
            &self.client,
            &self.key_pair,
            self.kid.as_deref(),
            &nonce,
            auth_url,
            None::<()>,
        )
        .await?;

        tracing::debug!(
            identifier = ?resp.identifier,
            status = resp.status.as_str(),
            "authorization response",
        );

        Ok(resp)
    }

    pub(crate) async fn trigger_challenge(
        &self,
        domain: &str,
        challenge_type: ChallengeType,
        url: &str,
    ) -> IoResult<()> {
        tracing::debug!(
            auth_uri = %url,
            domain = domain,
            challenge_type = %challenge_type,
            "trigger challenge",
        );

        let nonce = get_nonce(&self.client, &self.directory).await?;
        jose::request(
            &self.client,
            &self.key_pair,
            self.kid.as_deref(),
            &nonce,
            url,
            Some(serde_json::json!({})),
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn send_csr(&self, url: &str, csr: &[u8]) -> IoResult<NewOrderResponse> {
        tracing::debug!(url = %url, "send certificate request");

        let nonce = get_nonce(&self.client, &self.directory).await?;
        jose::request_json(
            &self.client,
            &self.key_pair,
            self.kid.as_deref(),
            &nonce,
            url,
            Some(CsrRequest {
                csr: URL_SAFE_NO_PAD.encode(csr),
            }),
        )
        .await
    }

    pub(crate) async fn obtain_certificate(&self, url: &str) -> IoResult<Vec<u8>> {
        tracing::debug!(url = %url, "send certificate request");

        let nonce = get_nonce(&self.client, &self.directory).await?;
        let resp = jose::request(
            &self.client,
            &self.key_pair,
            self.kid.as_deref(),
            &nonce,
            url,
            None::<()>,
        )
        .await?;

        Ok(resp
            .bytes()
            .await
            .map_err(|err| IoError::other(format!("failed to download certificate: {err}")))?
            .to_vec())
    }
}

async fn get_directory(client: &Client, directory_url: &str) -> IoResult<Directory> {
    tracing::debug!("loading directory");

    let resp = client
        .get(directory_url)
        .send()
        .await
        .map_err(|err| IoError::other(format!("failed to load directory: {err}")))?;

    if !resp.status().is_success() {
        return Err(IoError::other(format!(
            "failed to load directory: status = {}",
            resp.status()
        )));
    }

    let directory = resp
        .json::<Directory>()
        .await
        .map_err(|err| IoError::other(format!("failed to load directory: {err}")))?;

    tracing::debug!(
        new_nonce = ?directory.new_nonce,
        new_account = ?directory.new_account,
        new_order = ?directory.new_order,
        "directory loaded",
    );
    Ok(directory)
}

async fn get_nonce(client: &Client, directory: &Directory) -> IoResult<String> {
    tracing::debug!("creating nonce");

    let resp = client
        .get(&directory.new_nonce)
        .send()
        .await
        .map_err(|err| IoError::other(format!("failed to get nonce: {err}")))?;

    if !resp.status().is_success() {
        return Err(IoError::other(format!(
            "failed to load directory: status = {}",
            resp.status()
        )));
    }

    let nonce = resp
        .headers()
        .get("replay-nonce")
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string)
        .unwrap_or_default();

    tracing::debug!(nonce = nonce.as_str(), "nonce created");
    Ok(nonce)
}

async fn create_acme_account(
    client: &Client,
    directory: &Directory,
    key_pair: &KeyPair,
    contacts: Vec<String>,
) -> IoResult<String> {
    tracing::debug!("creating acme account");

    let nonce = get_nonce(client, directory).await?;
    let resp = jose::request(
        client,
        key_pair,
        None,
        &nonce,
        &directory.new_account,
        Some(NewAccountRequest {
            only_return_existing: false,
            terms_of_service_agreed: true,
            contacts,
        }),
    )
    .await?;
    let kid = resp
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string)
        .ok_or_else(|| IoError::other("unable to get account id"))?;

    tracing::debug!(kid = kid.as_str(), "account created");
    Ok(kid)
}
