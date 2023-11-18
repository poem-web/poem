use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use parking_lot::RwLock;
use tokio_rustls::rustls::{
    server::{ClientHello, ResolvesServerCert},
    sign::CertifiedKey,
};
use x509_parser::prelude::{FromDer, X509Certificate};

pub(crate) const ACME_TLS_ALPN_NAME: &[u8] = b"acme-tls/1";

/// Returns the number of seconds until the certificate expires or 0
/// if there's no certificate in the key.
pub fn seconds_until_expiry(cert: &CertifiedKey) -> i64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let expires_at = cert
        .cert
        .first()
        .and_then(|cert| X509Certificate::from_der(cert.as_ref()).ok())
        .map(|(_, cert)| cert.validity().not_after.timestamp())
        .unwrap_or(0);
    expires_at - now
}

/// Shared ACME key state.
#[derive(Default, Debug)]
pub struct ResolveServerCert {
    /// The current TLS certificate. Swap it with `Arc::write`.
    pub cert: RwLock<Option<Arc<CertifiedKey>>>,
    pub(crate) acme_keys: RwLock<HashMap<String, Arc<CertifiedKey>>>,
}

impl ResolveServerCert {
    pub(crate) fn is_expired(&self) -> bool {
        self.cert
            .read()
            .as_ref()
            .map(|cert| seconds_until_expiry(cert) < 60 * 60 * 12)
            .unwrap_or(true)
    }
}

impl ResolvesServerCert for ResolveServerCert {
    fn resolve(&self, client_hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        if client_hello
            .alpn()
            .and_then(|mut iter| iter.find(|alpn| *alpn == ACME_TLS_ALPN_NAME))
            .is_some()
        {
            return match client_hello.server_name() {
                None => None,
                Some(domain) => {
                    tracing::debug!(domain = domain, "load acme key");
                    match self.acme_keys.read().get(domain).cloned() {
                        Some(cert) => Some(cert),
                        None => {
                            tracing::error!(domain = domain, "acme key not found");
                            None
                        }
                    }
                }
            };
        };

        self.cert.read().as_ref().cloned()
    }
}
