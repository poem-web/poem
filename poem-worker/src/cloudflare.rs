use http::StatusCode;
use poem::{FromRequest, Request, RequestBody};
use serde::de::DeserializeOwned;
use worker::{Cf, TlsClientAuth};

pub struct CloudflareProperties(Cf);

impl CloudflareProperties {
    pub fn colo(&self) -> String {
        self.0.colo()
    }

    pub fn asn(&self) -> u32 {
        self.0.asn()
    }

    pub fn as_organization(&self) -> String {
        self.0.as_organization()
    }

    pub fn country(&self) -> Option<String> {
        self.0.country()
    }

    pub fn http_protocol(&self) -> String {
        self.0.http_protocol()
    }

    pub fn tls_cipher(&self) -> String {
        self.0.tls_cipher()
    }

    pub fn tls_client_auth(&self) -> Option<TlsClientAuth> {
        self.0.tls_client_auth()
    }

    pub fn tls_version(&self) -> String {
        self.0.tls_version()
    }

    pub fn city(&self) -> Option<String> {
        self.0.city()
    }

    pub fn continent(&self) -> Option<String> {
        self.0.continent()
    }

    pub fn coordinates(&self) -> Option<(f32, f32)> {
        self.0.coordinates()
    }

    pub fn postal_code(&self) -> Option<String> {
        self.0.postal_code()
    }

    pub fn metro_code(&self) -> Option<String> {
        self.0.metro_code()
    }

    pub fn region(&self) -> Option<String> {
        self.0.region()
    }

    pub fn region_code(&self) -> Option<String> {
        self.0.region_code()
    }

    pub fn timezone_name(&self) -> String {
        self.0.timezone_name()
    }

    pub fn is_eu_country(&self) -> bool {
        self.0.is_eu_country()
    }

    pub fn host_metadata<T: DeserializeOwned>(&self) -> Result<Option<T>, worker::Error> {
        self.0.host_metadata::<T>()
    }
}

impl<'a> FromRequest<'a> for CloudflareProperties {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self, poem::Error> {
        let cf = req.data::<Cf>().ok_or_else(|| {
            poem::Error::from_string(
                "failed to get incoming cloudflare properties",
                StatusCode::BAD_REQUEST,
            )
        })?;

        Ok(CloudflareProperties(cf.clone()))
    }
}
