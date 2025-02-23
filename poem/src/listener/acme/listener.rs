use std::{
    io::{Error as IoError, ErrorKind, Result as IoResult},
    sync::{Arc, Weak},
    time::{Duration, UNIX_EPOCH},
};

use http::uri::Scheme;
use rcgen::{
    Certificate, CertificateParams, CustomExtension, DistinguishedName, PKCS_ECDSA_P256_SHA256,
};
use tokio_rustls::{
    rustls::{
        crypto::ring::sign::any_ecdsa_type,
        pki_types::{CertificateDer, PrivateKeyDer},
        sign::CertifiedKey,
        ServerConfig,
    },
    server::TlsStream,
    TlsAcceptor,
};
use x509_parser::prelude::{FromDer, X509Certificate};

use crate::{
    listener::{
        acme::{
            client::AcmeClient,
            jose,
            resolver::{ResolveServerCert, ACME_TLS_ALPN_NAME},
            AutoCert, ChallengeType, Http01TokensMap,
        },
        Acceptor, HandshakeStream, Listener,
    },
    web::{LocalAddr, RemoteAddr},
};

pub(crate) async fn auto_cert_acceptor<T: Listener>(
    base_listener: T,
    cert_resolver: Arc<ResolveServerCert>,
    challenge_type: ChallengeType,
) -> IoResult<AutoCertAcceptor<T::Acceptor>> {
    let mut server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(cert_resolver);
    server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    if challenge_type == ChallengeType::TlsAlpn01 {
        server_config
            .alpn_protocols
            .push(ACME_TLS_ALPN_NAME.to_vec());
    }
    let acceptor = TlsAcceptor::from(Arc::new(server_config));
    Ok(AutoCertAcceptor {
        inner: base_listener.into_acceptor().await?,
        acceptor,
    })
}

/// A listener that uses the TLS cert provided by the cert resolver.
pub struct ResolvedCertListener<T> {
    inner: T,
    cert_resolver: Arc<ResolveServerCert>,
    challenge_type: ChallengeType,
}

impl<T> ResolvedCertListener<T> {
    /// Create a new `ResolvedCertListener`.
    pub fn new(
        inner: T,
        cert_resolver: Arc<ResolveServerCert>,
        challenge_type: ChallengeType,
    ) -> Self {
        Self {
            inner,
            cert_resolver,
            challenge_type,
        }
    }
}

impl<T: Listener> Listener for ResolvedCertListener<T> {
    type Acceptor = AutoCertAcceptor<T::Acceptor>;

    async fn into_acceptor(self) -> IoResult<Self::Acceptor> {
        auto_cert_acceptor(self.inner, self.cert_resolver, self.challenge_type).await
    }
}

/// A wrapper around an underlying listener which implements the ACME.
pub struct AutoCertListener<T> {
    inner: T,
    auto_cert: AutoCert,
}

impl<T> AutoCertListener<T> {
    pub(crate) fn new(inner: T, auto_cert: AutoCert) -> Self {
        Self { inner, auto_cert }
    }
}

impl<T: Listener> Listener for AutoCertListener<T> {
    type Acceptor = AutoCertAcceptor<T::Acceptor>;

    async fn into_acceptor(self) -> IoResult<Self::Acceptor> {
        let mut client = AcmeClient::try_new(
            &self.auto_cert.directory_url,
            self.auto_cert.contacts.clone(),
        )
        .await?;

        let (cache_certs, cert_key) = {
            let mut certs: Option<Vec<_>> = None;
            let mut key = None;

            if let Some(cache_cert) = &self.auto_cert.cache_cert {
                match rustls_pemfile::certs(&mut cache_cert.as_slice())
                    .collect::<Result<_, _>>()
                    .map_err(|err| IoError::new(ErrorKind::Other, format!("invalid pem: {err}")))
                {
                    Ok(c) => certs = Some(c),
                    Err(err) => {
                        tracing::warn!("failed to parse cached tls certificates: {}", err)
                    }
                };
            }

            if let Some(cache_key) = &self.auto_cert.cache_key {
                match rustls_pemfile::pkcs8_private_keys(&mut cache_key.as_slice())
                    .collect::<Result<Vec<_>, _>>()
                {
                    Ok(k) => key = k.into_iter().next(),
                    Err(err) => {
                        tracing::warn!("failed to parse cached private key: {}", err)
                    }
                };
            }

            (certs, key)
        };

        let cert_resolver = Arc::new(ResolveServerCert::default());

        if let (Some(certs), Some(key)) = (cache_certs, cert_key) {
            let certs = certs.into_iter().collect::<Vec<_>>();

            let expires_at = match certs
                .first()
                .and_then(|cert| X509Certificate::from_der(cert.as_ref()).ok())
                .map(|(_, cert)| cert.validity().not_after.timestamp())
                .map(|timestamp| UNIX_EPOCH + Duration::from_secs(timestamp as u64))
            {
                Some(expires_at) => chrono::DateTime::<chrono::Utc>::from(expires_at).to_string(),
                None => "unknown".to_string(),
            };

            tracing::debug!(
                expires_at = expires_at.as_str(),
                "using cached tls certificates"
            );
            *cert_resolver.cert.write() = Some(Arc::new(CertifiedKey::new(
                certs,
                any_ecdsa_type(&PrivateKeyDer::Pkcs8(key)).unwrap(),
            )));
        }

        let weak_cert_resolver = Arc::downgrade(&cert_resolver);
        let challenge_type = self.auto_cert.challenge_type;
        let domains = self.auto_cert.domains;
        let keys_for_http01 = self.auto_cert.keys_for_http01;
        let cache_path = self.auto_cert.cache_path;
        tokio::spawn(async move {
            while let Some(cert_resolver) = Weak::upgrade(&weak_cert_resolver) {
                if cert_resolver.is_expired() {
                    match issue_cert(
                        &mut client,
                        &cert_resolver,
                        &domains,
                        challenge_type,
                        keys_for_http01.as_ref(),
                    )
                    .await
                    {
                        Ok(res) => {
                            *cert_resolver.cert.write() = Some(res.rustls_key);
                            if let Some(cache_path) = &cache_path {
                                let pkey_path = cache_path.join("key.pem");
                                tracing::debug!(path =% pkey_path.display(), "write private key to cache path");
                                if let Err(err) = std::fs::write(pkey_path, res.private_pem) {
                                    tracing::error!(error =% err, "failed to write key pem to cache dir");
                                }
                                let cert_path = cache_path.join("cert.pem");
                                tracing::debug!(path =% cert_path.display(), "write certificate to cache path");
                                if let Err(err) = std::fs::write(cert_path, res.public_pem) {
                                    tracing::error!(error =% err, "failed to write cert pem to cache dir");
                                }
                            }
                        }
                        Err(err) => {
                            tracing::error!(error =% err, "failed to issue certificate");
                        }
                    }
                }
                tokio::time::sleep(Duration::from_secs(60 * 5)).await;
            }
        });
        auto_cert_acceptor(self.inner, cert_resolver, challenge_type).await
    }
}

/// A ACME acceptor.
pub struct AutoCertAcceptor<T> {
    inner: T,
    acceptor: TlsAcceptor,
}

impl<T: Acceptor> Acceptor for AutoCertAcceptor<T> {
    type Io = HandshakeStream<TlsStream<T::Io>>;

    fn local_addr(&self) -> Vec<LocalAddr> {
        self.inner.local_addr()
    }

    async fn accept(&mut self) -> IoResult<(Self::Io, LocalAddr, RemoteAddr, Scheme)> {
        let (stream, local_addr, remote_addr, _) = self.inner.accept().await?;
        let stream = HandshakeStream::new(self.acceptor.accept(stream));
        Ok((stream, local_addr, remote_addr, Scheme::HTTPS))
    }
}

fn gen_acme_cert(domain: &str, acme_hash: &[u8]) -> IoResult<CertifiedKey> {
    let mut params = CertificateParams::new(vec![domain.to_string()]);
    params.alg = &PKCS_ECDSA_P256_SHA256;
    params.custom_extensions = vec![CustomExtension::new_acme_identifier(acme_hash)];
    let cert = Certificate::from_params(params)
        .map_err(|_| IoError::new(ErrorKind::Other, "failed to generate acme certificate"))?;
    let key = any_ecdsa_type(&PrivateKeyDer::Pkcs8(
        cert.serialize_private_key_der().into(),
    ))
    .unwrap();
    Ok(CertifiedKey::new(
        vec![CertificateDer::from(cert.serialize_der().map_err(
            |_| IoError::new(ErrorKind::Other, "failed to serialize acme certificate"),
        )?)],
        key,
    ))
}

/// The result of [`issue_cert`] function.
pub struct IssueCertResult {
    pub private_pem: String,
    pub public_pem: Vec<u8>,
    pub rustls_key: Arc<CertifiedKey>,
}

/// Generate a new certificate via ACME protocol.  Returns the pub cert and
/// private key in PEM format, and the private key as a Rustls object.
///
/// It is up to the caller to make use of the returned certificate, this
/// function does nothing outside for the ACME protocol procedure.
pub async fn issue_cert<T: AsRef<str>>(
    client: &mut AcmeClient,
    resolver: &ResolveServerCert,
    domains: &[T],
    challenge_type: ChallengeType,
    keys_for_http01: Option<&Http01TokensMap>,
) -> IoResult<IssueCertResult> {
    tracing::debug!("issue certificate");
    let order_resp = client.new_order(domains).await?;

    // trigger challenge
    let mut valid = false;

    for i in 1..5 {
        let mut all_valid = true;

        for auth_url in &order_resp.authorizations {
            let resp = client.fetch_authorization(auth_url).await?;

            if resp.status == "valid" {
                continue;
            }

            all_valid = false;

            if resp.status == "pending" {
                let challenge = resp.find_challenge(challenge_type)?;

                match challenge_type {
                    ChallengeType::Http01 => {
                        if let Some(keys) = &keys_for_http01 {
                            let key_authorization =
                                jose::key_authorization(&client.key_pair, &challenge.token)?;
                            keys.insert(challenge.token.to_string(), key_authorization);
                        }
                    }
                    ChallengeType::TlsAlpn01 => {
                        let key_authorization_sha256 =
                            jose::key_authorization_sha256(&client.key_pair, &challenge.token)?;
                        let auth_key = gen_acme_cert(
                            &resp.identifier.value,
                            key_authorization_sha256.as_ref(),
                        )?;

                        resolver
                            .acme_keys
                            .write()
                            .insert(resp.identifier.value.to_string(), Arc::new(auth_key));
                    }
                }

                client
                    .trigger_challenge(&resp.identifier.value, challenge_type, &challenge.url)
                    .await?;
            } else if resp.status == "invalid" {
                return Err(IoError::new(
                    ErrorKind::Other,
                    format!(
                        "unable to authorize `{}`: {}",
                        resp.identifier.value,
                        resp.error
                            .as_ref()
                            .map(|problem| &*problem.detail)
                            .unwrap_or("unknown")
                    ),
                ));
            }
        }

        if all_valid {
            valid = true;
            break;
        }

        tokio::time::sleep(Duration::from_secs(i * 10)).await;
    }

    if !valid {
        return Err(IoError::new(
            ErrorKind::Other,
            "authorization failed too many times",
        ));
    }

    // send csr
    let mut params = CertificateParams::new(
        domains
            .iter()
            .map(|domain| domain.as_ref().to_string())
            .collect::<Vec<_>>(),
    );
    params.distinguished_name = DistinguishedName::new();
    params.alg = &PKCS_ECDSA_P256_SHA256;
    let cert = Certificate::from_params(params).map_err(|err| {
        IoError::new(
            ErrorKind::Other,
            format!("failed create certificate request: {err}"),
        )
    })?;
    let pk = any_ecdsa_type(&PrivateKeyDer::Pkcs8(
        cert.serialize_private_key_der().into(),
    ))
    .unwrap();
    let csr = cert.serialize_request_der().map_err(|err| {
        IoError::new(
            ErrorKind::Other,
            format!("failed to serialize request der {err}"),
        )
    })?;

    let order_resp = client.send_csr(&order_resp.finalize, &csr).await?;

    if order_resp.status == "invalid" {
        return Err(IoError::new(
            ErrorKind::Other,
            format!(
                "failed to request certificate: {}",
                order_resp
                    .error
                    .as_ref()
                    .map(|problem| &*problem.detail)
                    .unwrap_or("unknown")
            ),
        ));
    }

    if order_resp.status != "valid" {
        return Err(IoError::new(
            ErrorKind::Other,
            format!(
                "failed to request certificate: unexpected status `{}`",
                order_resp.status
            ),
        ));
    }

    // download certificate
    let acme_cert_pem = client
        .obtain_certificate(order_resp.certificate.as_ref().ok_or_else(|| {
            IoError::new(
                ErrorKind::Other,
                "invalid response: missing `certificate` url",
            )
        })?)
        .await?;
    let pkey_pem = cert.serialize_private_key_pem();
    let cert_chain = rustls_pemfile::certs(&mut acme_cert_pem.as_slice())
        .collect::<Result<_, _>>()
        .map_err(|err| IoError::new(ErrorKind::Other, format!("invalid pem: {err}")))?;
    let cert_key = CertifiedKey::new(cert_chain, pk);

    tracing::debug!("certificate obtained");

    Ok(IssueCertResult {
        private_pem: pkey_pem,
        public_pem: acme_cert_pem,
        rustls_key: Arc::new(cert_key),
    })
}
