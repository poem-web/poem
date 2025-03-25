use std::io::{Error as IoError, ErrorKind, Result as IoResult};

use ring::{
    rand::SystemRandom,
    signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair, KeyPair as _, Signature},
};

pub(crate) struct KeyPair(EcdsaKeyPair);

impl KeyPair {
    pub(crate) fn from_pkcs8(pkcs8: impl AsRef<[u8]>) -> IoResult<Self> {
        let rng = SystemRandom::new();
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref(), &rng)
            .map(KeyPair)
            .map_err(|_| IoError::new(ErrorKind::Other, "failed to load key pair"))
    }

    fn generate_pkcs8() -> IoResult<impl AsRef<[u8]>> {
        let alg = &ECDSA_P256_SHA256_FIXED_SIGNING;
        let rng = SystemRandom::new();
        EcdsaKeyPair::generate_pkcs8(alg, &rng)
            .map_err(|_| IoError::new(ErrorKind::Other, "failed to generate acme key pair"))
    }

    pub(crate) fn generate() -> IoResult<Self> {
        Self::from_pkcs8(Self::generate_pkcs8()?)
    }

    pub(crate) fn sign(&self, message: impl AsRef<[u8]>) -> IoResult<Signature> {
        self.0
            .sign(&SystemRandom::new(), message.as_ref())
            .map_err(|_| IoError::new(ErrorKind::Other, "failed to sign message"))
    }

    pub(crate) fn public_key(&self) -> &[u8] {
        self.0.public_key().as_ref()
    }
}
