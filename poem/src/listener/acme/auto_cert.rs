use std::{
    fmt::{self, Debug, Formatter},
    path::PathBuf,
    sync::Arc,
};

use http::Uri;

use crate::listener::acme::{builder::AutoCertBuilder, keypair::KeyPair};

/// ACME configuration
pub struct AutoCert {
    pub(crate) directory_url: Uri,
    pub(crate) domains: Vec<String>,
    pub(crate) key_pair: Arc<KeyPair>,
    pub(crate) cache_path: Option<PathBuf>,
    pub(crate) cache_cert: Option<Vec<u8>>,
    pub(crate) cache_key: Option<Vec<u8>>,
}

impl AutoCert {
    /// Create an ACME configuration builder.
    pub fn builder() -> AutoCertBuilder {
        AutoCertBuilder::new()
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
