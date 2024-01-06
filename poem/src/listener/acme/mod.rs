//! Types for ACME.
//!
//! Reference: <https://datatracker.ietf.org/doc/html/rfc8555>
//! Reference: <https://datatracker.ietf.org/doc/html/rfc8737>

mod auto_cert;
mod builder;
mod client;
mod endpoint;
mod jose;
mod keypair;
mod listener;
mod protocol;
mod resolver;

pub use auto_cert::AutoCert;
pub use builder::AutoCertBuilder;
pub use client::AcmeClient;
pub use endpoint::{Http01Endpoint, Http01TokensMap};
pub use listener::{issue_cert, AutoCertAcceptor, AutoCertListener, ResolvedCertListener};
pub use protocol::ChallengeType;
pub use resolver::{seconds_until_expiry, ResolveServerCert};

/// Let's Encrypt production directory url
pub const LETS_ENCRYPT_PRODUCTION: &str = "https://acme-v02.api.letsencrypt.org/directory";

/// Let's Encrypt staging directory url
pub const LETS_ENCRYPT_STAGING: &str = "https://acme-staging-v02.api.letsencrypt.org/directory";
