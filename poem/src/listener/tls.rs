use std::sync::Arc;

use tokio::io::{Error as IoError, ErrorKind, Result as IoResult};
use tokio_rustls::{
    rustls::{
        AllowAnyAnonymousOrAuthenticatedClient, AllowAnyAuthenticatedClient, NoClientAuth,
        RootCertStore, ServerConfig,
    },
    server::TlsStream,
};

use crate::listener::{Acceptor, Listener};

#[cfg_attr(docsrs, doc(cfg(feature = "tls")))]
enum TlsClientAuth {
    Off,
    Optional(Vec<u8>),
    Required(Vec<u8>),
}

/// TLS Config.
#[cfg_attr(docsrs, doc(cfg(feature = "tls")))]
pub struct TlsConfig {
    cert: Vec<u8>,
    key: Vec<u8>,
    client_auth: TlsClientAuth,
    ocsp_resp: Vec<u8>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl TlsConfig {
    /// Create a new tls config object.
    pub fn new() -> Self {
        Self {
            cert: Vec::new(),
            key: Vec::new(),
            client_auth: TlsClientAuth::Off,
            ocsp_resp: Vec::new(),
        }
    }

    /// Sets the certificates.
    pub fn cert(mut self, cert: impl Into<Vec<u8>>) -> Self {
        self.cert = cert.into();
        self
    }

    /// Sets the private key.
    pub fn key(mut self, key: impl Into<Vec<u8>>) -> Self {
        self.key = key.into();
        self
    }

    /// Sets the trust anchor for optional client authentication.
    pub fn client_auth_optional(mut self, trust_anchor: impl Into<Vec<u8>>) -> Self {
        self.client_auth = TlsClientAuth::Optional(trust_anchor.into());
        self
    }

    /// Sets the trust anchor for required client authentication.
    pub fn client_auth_required(mut self, trust_anchor: impl Into<Vec<u8>>) -> Self {
        self.client_auth = TlsClientAuth::Required(trust_anchor.into());
        self
    }

    /// Sets the DER-encoded OCSP response.
    pub fn ocsp_resp(mut self, ocsp_resp: impl Into<Vec<u8>>) -> Self {
        self.ocsp_resp = ocsp_resp.into();
        self
    }
}

/// A wrapper around an underlying listener which implements the TLS or SSL
/// protocol.
///
/// NOTE: You cannot create it directly and should use the
/// [`tls`](crate::listener::Listener::tls) method to create it, because it
/// needs to wrap a underlying listener.
#[cfg_attr(docsrs, doc(cfg(feature = "tls")))]
pub struct TlsListener<T> {
    config: TlsConfig,
    inner: T,
}

impl<T: Listener> TlsListener<T> {
    pub(crate) fn new(config: TlsConfig, inner: T) -> Self {
        Self { config, inner }
    }
}

#[async_trait::async_trait]
impl<T: Listener> Listener for TlsListener<T> {
    type Acceptor = TlsAcceptor<T::Acceptor>;

    async fn into_acceptor(self) -> IoResult<Self::Acceptor> {
        let cert = tokio_rustls::rustls::internal::pemfile::certs(&mut self.config.cert.as_slice())
            .map_err(|_| IoError::new(ErrorKind::Other, "failed to parse tls certificates"))?;
        let key = {
            let mut pkcs8 = tokio_rustls::rustls::internal::pemfile::pkcs8_private_keys(
                &mut self.config.key.as_slice(),
            )
            .map_err(|_| IoError::new(ErrorKind::Other, "failed to parse tls private keys"))?;
            if !pkcs8.is_empty() {
                pkcs8.remove(0)
            } else {
                let mut rsa = tokio_rustls::rustls::internal::pemfile::rsa_private_keys(
                    &mut self.config.key.as_slice(),
                )
                .map_err(|_| IoError::new(ErrorKind::Other, "failed to parse tls private keys"))?;

                if !rsa.is_empty() {
                    rsa.remove(0)
                } else {
                    return Err(IoError::new(
                        ErrorKind::Other,
                        "failed to parse tls private keys",
                    ));
                }
            }
        };

        fn read_trust_anchor(mut trust_anchor: &[u8]) -> IoResult<RootCertStore> {
            let mut store = RootCertStore::empty();
            if let Ok((0, _)) | Err(()) = store.add_pem_file(&mut trust_anchor) {
                Err(IoError::new(
                    ErrorKind::Other,
                    "failed to parse tls trust anchor",
                ))
            } else {
                Ok(store)
            }
        }

        let client_auth = match self.config.client_auth {
            TlsClientAuth::Off => NoClientAuth::new(),
            TlsClientAuth::Optional(trust_anchor) => {
                AllowAnyAnonymousOrAuthenticatedClient::new(read_trust_anchor(&trust_anchor)?)
            }
            TlsClientAuth::Required(trust_anchor) => {
                AllowAnyAuthenticatedClient::new(read_trust_anchor(&trust_anchor)?)
            }
        };

        let mut config = ServerConfig::new(client_auth);
        config
            .set_single_cert_with_ocsp_and_sct(cert, key, self.config.ocsp_resp, Vec::new())
            .map_err(|err| IoError::new(ErrorKind::Other, err.to_string()))?;
        config.set_protocols(&["h2".into(), "http/1.1".into()]);

        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        Ok(TlsAcceptor {
            acceptor,
            inner: self.inner.into_acceptor().await?,
        })
    }
}

/// A TLS or SSL protocol acceptor.
#[cfg_attr(docsrs, doc(cfg(feature = "tls")))]
pub struct TlsAcceptor<T> {
    acceptor: tokio_rustls::TlsAcceptor,
    inner: T,
}

#[async_trait::async_trait]
impl<T: Acceptor> Acceptor for TlsAcceptor<T> {
    type Addr = T::Addr;
    type Io = TlsStream<T::Io>;

    fn local_addr(&self) -> IoResult<Vec<Self::Addr>> {
        self.inner.local_addr()
    }

    async fn accept(&mut self) -> IoResult<(Self::Io, Self::Addr)> {
        let (stream, addr) = self.inner.accept().await?;
        let stream = self.acceptor.accept(stream).await?;
        Ok((stream, addr))
    }
}

#[cfg(test)]
mod tests {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    };
    use tokio_rustls::rustls::ClientConfig;

    use super::*;
    use crate::listener::TcpListener;

    const CERT: &str = r#"
-----BEGIN CERTIFICATE-----
MIIEADCCAmigAwIBAgICAcgwDQYJKoZIhvcNAQELBQAwLDEqMCgGA1UEAwwhcG9u
eXRvd24gUlNBIGxldmVsIDIgaW50ZXJtZWRpYXRlMB4XDTE2MTIxMDE3NDIzM1oX
DTIyMDYwMjE3NDIzM1owGTEXMBUGA1UEAwwOdGVzdHNlcnZlci5jb20wggEiMA0G
CSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQC1YDz66+7VD4DL1+/sVHMQ+BbDRgmD
OQlX++mfW8D3QNQm/qDBEbu7T7qqdc9GKDar4WIzBN8SBkzM1EjMGwNnZPV/Tfz0
qUAR1L/7Zzf1GaFZvWXgksyUpfwvmprH3Iy/dpkETwtPthpTPNlui3hZnm/5kkjR
RWg9HmID4O04Ld6SK313v2ZgrPZbkKvbqlqhUnYWjL3blKVGbpXIsuZzEU9Ph+gH
tPcEhZpFsM6eLe+2TVscIrycMEOTXqAAmO6zZ9sQWtfllu3CElm904H6+jA/9Leg
al72pMmkYr8wWniqDDuijXuCPlVx5EDFFyxBmW18UeDEQaKV3kNfelaTAgMBAAGj
gb4wgbswDAYDVR0TAQH/BAIwADALBgNVHQ8EBAMCBsAwHQYDVR0OBBYEFIYhJkVy
AAKT6cY/ruH1Eu+NNxteMEIGA1UdIwQ7MDmAFNwuPy4Do//Sm5CZDrocHWTrNr96
oR6kHDAaMRgwFgYDVQQDDA9wb255dG93biBSU0EgQ0GCAXswOwYDVR0RBDQwMoIO
dGVzdHNlcnZlci5jb22CFXNlY29uZC50ZXN0c2VydmVyLmNvbYIJbG9jYWxob3N0
MA0GCSqGSIb3DQEBCwUAA4IBgQCWV76jfQDZKtfmj45fTwZzoe/PxjWPRbAvSEnt
LRHrPhqQfpMLqpun8uu/w86mHiR/AmiAySMu3zivW6wfGzlRWLi/zCyO6r9LGsgH
bNk5CF642cdZFvn1SiSm1oGXQrolIpcyXu88nUpt74RnY4ETCC1dRQKqxsYufe5T
DOmTm3ChinNW4QRG3yvW6DVuyxVAgZvofyKJOsM3GO6oogIM41aBqZ3UTwmIwp6D
oISdiATslFOzYzjnyXNR8DG8OOkv1ehWuyb8x+hQCZAuogQOWYtCSd6k3kKgd0EM
4CWbt1XDV9ZJwBf2uxZeKuCu/KIy9auNtijAwPsUv9qxuzko018zhl3lWm5p2Sqw
O7fFshU3A6df8hMw7ST6/tgFY7geT88U4iJhfWMwr/CZSRSVMXhTyJgbLIXxKYZj
Ym5v4NAIQP6hI4HixzQaYgrhW6YX6myk+emMjQLRJHT8uHvmT7fuxMJVWWgsCkr1
C75pRQEagykN/Uzr5e6Tm8sVu88=
-----END CERTIFICATE-----
"#;

    const CHAIN: &str = r#"
-----BEGIN CERTIFICATE-----
MIIGnzCCAoegAwIBAgIBezANBgkqhkiG9w0BAQsFADAaMRgwFgYDVQQDDA9wb255
dG93biBSU0EgQ0EwHhcNMTYxMjEwMTc0MjMzWhcNMjYxMjA4MTc0MjMzWjAsMSow
KAYDVQQDDCFwb255dG93biBSU0EgbGV2ZWwgMiBpbnRlcm1lZGlhdGUwggGiMA0G
CSqGSIb3DQEBAQUAA4IBjwAwggGKAoIBgQDnfb7vaJbaHEyVTflswWhmHqx5W0NO
KyKbDp2zXEJwDO+NDJq6i1HGnFd/vO4LyjJBU1wUsKtE+m55cfRmUHVuZ2w4n/VF
p7Z7n+SNuvJNcrzDxyKVy4GIZ39zQePnniqtLqXh6eI8Ow6jiMgVxC/wbWcVLKv6
4RM+2fLjJAC9b27QfjhOlMKVeMOEvPrrpjLSauaHAktQPhuzIAwzxM0+KnvDkWWy
NVqAV/lq6fSO/9vJRhM4E2nxo6yqi7qTdxVxMmKsNn7L6HvjQgx+FXziAUs55Qd9
cP7etCmPmoefkcgdbxDOIKH8D+DvfacZwngqcnr/q96Ff4uJ13d2OzR1mWVSZ2hE
JQt/BbZBANciqu9OZf3dj6uOOXgFF705ak0GfLtpZpc29M+fVnknXPDSiKFqjzOO
KL+SRGyuNc9ZYjBKkXPJ1OToAs6JSvgDxfOfX0thuo2rslqfpj2qCFugsRIRAqvb
eyFwg+BPM/P/EfauXlAcQtBF04fOi7xN2okCAwEAAaNeMFwwHQYDVR0OBBYEFNwu
Py4Do//Sm5CZDrocHWTrNr96MCAGA1UdJQEB/wQWMBQGCCsGAQUFBwMBBggrBgEF
BQcDAjAMBgNVHRMEBTADAQH/MAsGA1UdDwQEAwIB/jANBgkqhkiG9w0BAQsFAAOC
BAEAMHZpBqDIUAVFZNw4XbuimXQ4K8q4uePrLGHLb4F/gHbr8kYrU4H+cy4l+xXf
2dlEBdZoqjSF7uXzQg5Fd8Ff3ZgutXd1xeUJnxo0VdpKIhqeaTPqhffC2X6FQQH5
KrN7NVWQSnUhPNpBFELpmdpY1lHigFW7nytYj0C6VJ4QsbqhfW+n/t+Zgqtfh/Od
ZbclzxFwMM55zRA2HP6IwXS2+d61Jk/RpDHTzhWdjGH4906zGNNMa7slHpCTA9Ju
TrtjEAGt2PBSievBJOHZW80KVAoEX2n9B3ZABaz+uX0VVZG0D2FwhPpUeA57YiXu
qiktZR4Ankph3LabXp4IlAX16qpYsEW8TWE/HLreeqoM0WDoI6rF9qnTpV2KWqBf
ziMYkfSkT7hQ2bWc493lW+QwSxCsuBsDwlrCwAl6jFSf1+jEQx98/8n9rDNyD9dL
PvECmtF30WY98nwZ9/kO2DufQrd0mwSHcIT0pAwl5fimpkwTjj+TTbytO3M4jK5L
tuIzsViQ95BmJQ3XuLdkQ/Ug8rpECYRX5fQX1qXkkvl920ohpKqKyEji1OmfmJ0Z
tZChaEcu3Mp3U+gD4az2ogmle3i/Phz8ZEPFo4/21G5Qd72z0lBgaQIeyyCk5MHt
Yg0vA7X0/w4bz+OJv5tf7zJsPCYSprr+c/7YUJk9Fqu6+g9ZAavI99xFKdGhz4Og
w0trnKNCxYc6+NPopTDbXuY+fo4DK7C0CSae5sKs7013Ne6w4KvgfLKpvlemkGfg
ZA3+1FMXVfFIEH7Cw9cx6F02Sr3k1VrU68oM3wH5nvTUkELOf8nRMlzliQjVCpKB
yFSe9dzRVSFEbMDxChiEulGgNUHj/6wwpg0ZmCwPRHutppT3jkfEqizN5iHb69GH
k6kol6knJofkaL656Q3Oc9o0ZrMlFh1RwmOvAk5fVK0/CV88/phROz2Wdmy5Bz4a
t0vzqFWA54y6+9EEVoOk9SU0CYfpGtpX4URjLK1EUG/l+RR3366Uee6TPrtEZ9cg
56VQMxhSaRNAvJ6DfiSuscSCNJzwuXaMXSZydGYnnP9Tb9p6c1uy1sXdluZkBIcK
CgC+gdDMSNlDn9ghc4xZGkuA8bjzfAYuRuGKmfTt8uuklkjw2b9w3SHjC4/Cmd2W
cFRnzfg2oL6e78hNg2ZGgsLzvb6Lu6/5IhXCO7RitzYf2+HLBbc+YLFsnG3qeGe1
28yGnXOQd97Cr4+IzFucVy/33gMQkesNUSDFJSq1gE/hGrMgTTMQJ7yC3PRqg0kG
tpqTyKNdM0g1adxlR1qfDPvpUBApkgBbySnMyWEr5+tBuoHUtH2m49oV9YD4odMJ
yJjlGxituO/YNN6O8oANlraG1Q==
-----END CERTIFICATE-----
-----BEGIN CERTIFICATE-----
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
DLxargy39i2aQGg=
-----END CERTIFICATE-----
"#;

    const KEY: &str = r#"
-----BEGIN RSA PRIVATE KEY-----
MIIEogIBAAKCAQEAtWA8+uvu1Q+Ay9fv7FRzEPgWw0YJgzkJV/vpn1vA90DUJv6g
wRG7u0+6qnXPRig2q+FiMwTfEgZMzNRIzBsDZ2T1f0389KlAEdS/+2c39RmhWb1l
4JLMlKX8L5qax9yMv3aZBE8LT7YaUzzZbot4WZ5v+ZJI0UVoPR5iA+DtOC3ekit9
d79mYKz2W5Cr26paoVJ2Foy925SlRm6VyLLmcxFPT4foB7T3BIWaRbDOni3vtk1b
HCK8nDBDk16gAJjus2fbEFrX5ZbtwhJZvdOB+vowP/S3oGpe9qTJpGK/MFp4qgw7
oo17gj5VceRAxRcsQZltfFHgxEGild5DX3pWkwIDAQABAoIBAFDTazlSbGML/pRY
TTWeyIw2UkaA7npIr45C13BJfitw+1nJPK/tDCDDveZ6i3yzLPHZhV5A/HtWzWC1
9R7nptOrnO83PNN2nPOVQFxzOe+ClXGdQkoagQp5EXHRTspj0WD9I+FUrDDAcOjJ
BAgMJPyi6zlnZAXGDVa3NGyQDoZqwU2k36L4rEsJIkG0NVurZhpiCexNkkf32495
TOINQ0iKdfJ4iZoEYQ9G+x4NiuAJRCHuIcH76SNfT+Uv3wX0ut5EFPtflnvtdgcp
QVcoKwYdO0+mgO5xqWlBcsujSvgBdiNAGnAxKHWiEaacuIJi4+yYovyEebP6QI2X
Zg/U2wkCgYEA794dE5CPXLOmv6nioVC/ubOESk7vjSlEka/XFbKr4EY794YEqrB1
8TUqg09Bn3396AS1e6P2shr3bxos5ybhOxDGSLnJ+aC0tRFjd1BPKnA80vZM7ggt
5cjmdD5Zp0tIQTIAAYU5bONQOwj0ej4PE7lny26eLa5vfvCwlrD+rM0CgYEAwZMN
W/5PA2A+EM08IaHic8my0dCunrNLF890ouZnDG99SbgMGvvEsGIcCP1sai702hNh
VgGDxCz6/HUy+4O4YNFVtjY7uGEpfIEcEI7CsLQRP2ggWEFxThZtnEtO8PbM3J/i
qcS6njHdE+0XuCjgZwGgva5xH2pkWFzw/AIpEN8CgYB2HOo2axWc8T2n3TCifI+c
EqCOsqXU3cBM+MgxgASQcCUxMkX0AuZguuxPMmS+85xmdoMi+c8NTqgOhlYcEJIR
sqXgw9OH3zF8g6513w7Md+4Ld4rUHyTypGWOUfF1pmVS7RsBpKdtTdWA7FzuIMbt
0HsiujqbheyTFlPuMAOH9QKBgBWS1gJSrWuq5j/pH7J/4EUXTZ6kq1F0mgHlVRJy
qzlvk38LzA2V0a32wTkfRV3wLcnALzDuqkjK2o4YYb42R+5CZlMQaEd8TKtbmE0g
HAKljuaKLFCpun8BcOXiXsHsP5i3GQPisQnAdOsrmWEk7R2NyORa9LCToutWMGVl
uD3xAoGAA183Vldm+m4KPsKS17t8MbwBryDXvowGzruh/Z+PGA0spr+ke4XxwT1y
kMMP1+5flzmjlAf4+W8LehKuVqvQoMlPn5UVHmSxQ7cGx/O/o6Gbn8Q25/6UT+sM
B1Y0rlLoKG62pnkeXp1O4I57gnClatWRg5qw11a8V8e3jvDKIYM=
-----END RSA PRIVATE KEY-----
"#;

    #[tokio::test]
    async fn tcp_listener() {
        let listener =
            TcpListener::bind("127.0.0.1:8081").tls(TlsConfig::new().key(KEY).cert(CERT));
        let mut acceptor = listener.into_acceptor().await.unwrap();

        tokio::spawn(async move {
            let mut config = ClientConfig::new();
            config
                .root_store
                .add_pem_file(&mut CHAIN.as_bytes())
                .unwrap();

            let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
            let domain = webpki::DNSNameRef::try_from_ascii_str("testserver.com").unwrap();
            let stream = TcpStream::connect("127.0.0.1:8081").await.unwrap();
            let mut stream = connector.connect(domain, stream).await.unwrap();
            stream.write_i32(10).await.unwrap();
        });

        let (mut stream, _) = acceptor.accept().await.unwrap();
        assert_eq!(stream.read_i32().await.unwrap(), 10);
    }
}
