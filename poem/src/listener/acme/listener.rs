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
        sign::{any_ecdsa_type, CertifiedKey},
        PrivateKey, ServerConfig,
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
            AutoCert, ChallengeType,
        },
        Acceptor, HandshakeStream, Listener,
    },
    web::{LocalAddr, RemoteAddr},
};

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

#[async_trait::async_trait]
impl<T: Listener> Listener for AutoCertListener<T> {
    type Acceptor = AutoCertAcceptor<T::Acceptor>;

    async fn into_acceptor(self) -> IoResult<Self::Acceptor> {
        let mut client = AcmeClient::try_new(
            &self.auto_cert.directory_url,
            self.auto_cert.key_pair.clone(),
            self.auto_cert.contacts.clone(),
        )
        .await?;

        let (cache_certs, cert_key) = {
            let mut certs = None;
            let mut key = None;

            if let Some(cache_cert) = &self.auto_cert.cache_cert {
                match rustls_pemfile::certs(&mut cache_cert.as_slice()) {
                    Ok(c) => certs = Some(c),
                    Err(err) => {
                        tracing::warn!("failed to parse cached tls certificates: {}", err)
                    }
                };
            }

            if let Some(cache_key) = &self.auto_cert.cache_key {
                match rustls_pemfile::pkcs8_private_keys(&mut cache_key.as_slice()) {
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
            let certs = certs
                .into_iter()
                .map(tokio_rustls::rustls::Certificate)
                .collect::<Vec<_>>();

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
                any_ecdsa_type(&PrivateKey(key)).unwrap(),
            )));
        }

        let weak_cert_resolver = Arc::downgrade(&cert_resolver);
        let mut server_config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_cert_resolver(cert_resolver);

        server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        if self.auto_cert.challenge_type == ChallengeType::TlsAlpn01 {
            server_config
                .alpn_protocols
                .push(ACME_TLS_ALPN_NAME.to_vec());
        }

        let acceptor = TlsAcceptor::from(Arc::new(server_config));
        let auto_cert = self.auto_cert;

        tokio::spawn(async move {
            while let Some(cert_resolver) = Weak::upgrade(&weak_cert_resolver) {
                if cert_resolver.is_expired() {
                    if let Err(err) = issue_cert(&mut client, &auto_cert, &cert_resolver).await {
                        tracing::error!(error = %err, "failed to issue certificate");
                    }
                }
                tokio::time::sleep(Duration::from_secs(60 * 5)).await;
            }
        });

        Ok(AutoCertAcceptor {
            inner: self.inner.into_acceptor().await?,
            acceptor,
        })
    }
}

/// A ACME acceptor.
pub struct AutoCertAcceptor<T> {
    inner: T,
    acceptor: TlsAcceptor,
}

#[async_trait::async_trait]
impl<T: Acceptor> Acceptor for AutoCertAcceptor<T> {
    type Io = HandshakeStream<TlsStream<T::Io>>;

    fn local_addr(&self) -> Vec<LocalAddr> {
        self.inner.local_addr()
    }

    async fn accept(&mut self) -> IoResult<(Self::Io, LocalAddr, RemoteAddr, Scheme)> {
        let (stream, local_addr, remote_addr, _) = self.inner.accept().await?;
        let stream = HandshakeStream::new(self.acceptor.accept(stream));
        return Ok((stream, local_addr, remote_addr, Scheme::HTTPS));
    }
}

fn gen_acme_cert(domain: &str, acme_hash: &[u8]) -> IoResult<CertifiedKey> {
    let mut params = CertificateParams::new(vec![domain.to_string()]);
    params.alg = &PKCS_ECDSA_P256_SHA256;
    params.custom_extensions = vec![CustomExtension::new_acme_identifier(acme_hash)];
    let cert = Certificate::from_params(params)
        .map_err(|_| IoError::new(ErrorKind::Other, "failed to generate acme certificate"))?;
    let key = any_ecdsa_type(&PrivateKey(cert.serialize_private_key_der())).unwrap();
    Ok(CertifiedKey::new(
        vec![tokio_rustls::rustls::Certificate(
            cert.serialize_der().map_err(|_| {
                IoError::new(ErrorKind::Other, "failed to serialize acme certificate")
            })?,
        )],
        key,
    ))
}

async fn issue_cert(
    client: &mut AcmeClient,
    auto_cert: &AutoCert,
    resolver: &ResolveServerCert,
) -> IoResult<()> {
    tracing::debug!("issue certificate");

    let order_resp = client.new_order(&auto_cert.domains).await?;

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
                let challenge = resp.find_challenge(auto_cert.challenge_type)?;

                match auto_cert.challenge_type {
                    ChallengeType::Http01 => {
                        if let Some(keys) = &auto_cert.keys_for_http01 {
                            let mut keys = keys.write();
                            let key_authorization =
                                jose::key_authorization(&auto_cert.key_pair, &challenge.token)?;
                            keys.insert(challenge.token.to_string(), key_authorization);
                        }
                    }
                    ChallengeType::TlsAlpn01 => {
                        let key_authorization_sha256 =
                            jose::key_authorization_sha256(&auto_cert.key_pair, &challenge.token)?;
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
                    .trigger_challenge(
                        &resp.identifier.value,
                        auto_cert.challenge_type,
                        &challenge.url,
                    )
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
    let mut params = CertificateParams::new(auto_cert.domains.clone());
    params.distinguished_name = DistinguishedName::new();
    params.alg = &PKCS_ECDSA_P256_SHA256;
    let cert = Certificate::from_params(params).map_err(|err| {
        IoError::new(
            ErrorKind::Other,
            format!("failed create certificate request: {err}"),
        )
    })?;
    let pk = any_ecdsa_type(&PrivateKey(cert.serialize_private_key_der())).unwrap();
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
        .map_err(|err| IoError::new(ErrorKind::Other, format!("invalid pem: {err}")))?
        .into_iter()
        .map(tokio_rustls::rustls::Certificate)
        .collect();
    let cert_key = CertifiedKey::new(cert_chain, pk);

    *resolver.cert.write() = Some(Arc::new(cert_key));

    tracing::debug!("certificate obtained");

    if let Some(cache_path) = &auto_cert.cache_path {
        let pkey_path = cache_path.join("key.pem");
        tracing::debug!(path = %pkey_path.display(), "write private key to cache path");
        std::fs::write(pkey_path, pkey_pem)?;

        let cert_path = cache_path.join("cert.pem");
        tracing::debug!(path = %cert_path.display(), "write certificate to cache path");
        std::fs::write(cert_path, acme_cert_pem)?;
    }

    Ok(())
}
