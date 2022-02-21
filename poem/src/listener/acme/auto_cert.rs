use std::{
    collections::HashMap,
    fmt::{self, Debug, Formatter},
    path::PathBuf,
    sync::Arc,
};

use http::Uri;
use parking_lot::RwLock;

use crate::listener::acme::{
    builder::AutoCertBuilder, endpoint::Http01Endpoint, keypair::KeyPair, ChallengeType,
};

/// ACME configuration
pub struct AutoCert {
    pub(crate) directory_url: Uri,
    pub(crate) domains: Vec<String>,
    pub(crate) key_pair: Arc<KeyPair>,
    pub(crate) challenge_type: ChallengeType,
    pub(crate) keys_for_http01: Option<Arc<RwLock<HashMap<String, String>>>>,
    pub(crate) cache_path: Option<PathBuf>,
    pub(crate) cache_cert: Option<Vec<u8>>,
    pub(crate) cache_key: Option<Vec<u8>>,
}

impl AutoCert {
    /// Create an ACME configuration builder.
    pub fn builder() -> AutoCertBuilder {
        AutoCertBuilder::new()
    }

    /// Create an endpoint for HTTP-01 challenge
    ///
    /// Reference: <https://letsencrypt.org/docs/challenge-types/#http-01-challenge>
    ///
    /// # Panics
    ///
    /// Panic if current challenge type is not [`ChallengeType::Http01`].
    pub fn http_01_endpoint(&self) -> Http01Endpoint {
        if let Some(keys_for_http01) = &self.keys_for_http01 {
            Http01Endpoint {
                keys: keys_for_http01.clone(),
            }
        } else {
            panic!("current challenge type is not `HTTP-01`");
        }
    }
}

impl Debug for AutoCert {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("AutoCert")
            .field("directory_url", &self.directory_url)
            .field("domains", &self.domains)
            .field("cache_path", &self.cache_path)
            .finish()
    }
}
