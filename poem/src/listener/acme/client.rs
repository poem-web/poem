use std::{
    io::{Error as IoError, ErrorKind, Result as IoResult},
    sync::Arc,
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use http::{header, Uri};
use hyper::{client::HttpConnector, Client};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};

use crate::{
    listener::acme::{
        jose,
        keypair::KeyPair,
        protocol::{
            CsrRequest, Directory, FetchAuthorizationResponse, Identifier, NewAccountRequest,
            NewOrderRequest, NewOrderResponse,
        },
        ChallengeType,
    },
    Body,
};

pub(crate) struct AcmeClient {
    client: Client<HttpsConnector<HttpConnector>>,
    directory: Directory,
    key_pair: Arc<KeyPair>,
    contacts: Vec<String>,
    kid: Option<String>,
}

impl AcmeClient {
    pub(crate) async fn try_new(
        directory_url: &Uri,
        key_pair: Arc<KeyPair>,
        contacts: Vec<String>,
    ) -> IoResult<Self> {
        let client_builder = HttpsConnectorBuilder::new();
        #[cfg(feature = "acme-native-roots")]
        let client_builder1 = client_builder.with_native_roots();
        #[cfg(all(feature = "acme-webpki-roots", not(feature = "acme-native-roots")))]
        let client_builder1 = client_builder.with_webpki_roots();
        let client =
            Client::builder().build(client_builder1.https_or_http().enable_http1().build());
        let directory = get_directory(&client, directory_url).await?;
        Ok(Self {
            client,
            directory,
            key_pair,
            contacts,
            kid: None,
        })
    }

    pub(crate) async fn new_order(&mut self, domains: &[String]) -> IoResult<NewOrderResponse> {
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
                        value: domain.to_string(),
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
        auth_url: &Uri,
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
        url: &Uri,
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

    pub(crate) async fn send_csr(&self, url: &Uri, csr: &[u8]) -> IoResult<NewOrderResponse> {
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

    pub(crate) async fn obtain_certificate(&self, url: &Uri) -> IoResult<Vec<u8>> {
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

        resp.into_body().into_vec().await.map_err(|err| {
            IoError::new(
                ErrorKind::Other,
                format!("failed to download certificate: {err}"),
            )
        })
    }
}

async fn get_directory(
    client: &Client<HttpsConnector<HttpConnector>>,
    directory_url: &Uri,
) -> IoResult<Directory> {
    tracing::debug!("loading directory");

    let resp = client.get(directory_url.clone()).await.map_err(|err| {
        IoError::new(ErrorKind::Other, format!("failed to load directory: {err}"))
    })?;

    if !resp.status().is_success() {
        return Err(IoError::new(
            ErrorKind::Other,
            format!("failed to load directory: status = {}", resp.status()),
        ));
    }

    let directory = Body(resp.into_body())
        .into_json::<Directory>()
        .await
        .map_err(|err| {
            IoError::new(ErrorKind::Other, format!("failed to load directory: {err}"))
        })?;

    tracing::debug!(
        new_nonce = ?directory.new_nonce,
        new_account = ?directory.new_account,
        new_order = ?directory.new_order,
        "directory loaded",
    );
    Ok(directory)
}

async fn get_nonce(
    client: &Client<HttpsConnector<HttpConnector>>,
    directory: &Directory,
) -> IoResult<String> {
    tracing::debug!("creating nonce");

    let resp = client
        .get(directory.new_nonce.clone())
        .await
        .map_err(|err| IoError::new(ErrorKind::Other, format!("failed to get nonce: {err}")))?;

    if !resp.status().is_success() {
        return Err(IoError::new(
            ErrorKind::Other,
            format!("failed to load directory: status = {}", resp.status()),
        ));
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
    client: &Client<HttpsConnector<HttpConnector>>,
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
        .header(header::LOCATION)
        .ok_or_else(|| IoError::new(ErrorKind::Other, "unable to get account id"))?
        .to_string();

    tracing::debug!(kid = kid.as_str(), "account created");
    Ok(kid)
}
