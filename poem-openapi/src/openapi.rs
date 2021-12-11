use poem::{
    endpoint::{make_sync, BoxEndpoint},
    middleware::CookieJarManager,
    web::cookie::CookieKey,
    Endpoint, EndpointExt, IntoEndpoint, Request, Response, Route,
};

use crate::{
    base::UrlQuery,
    registry::{Document, MetaInfo, MetaLicense, MetaServer, Registry},
    OpenApi,
};

/// An object representing a Server.
#[derive(Debug, Clone)]
pub struct ServerObject {
    url: String,
    description: Option<String>,
}

impl From<String> for ServerObject {
    fn from(url: String) -> Self {
        Self::new(url)
    }
}

impl From<&str> for ServerObject {
    fn from(url: &str) -> Self {
        Self::new(url.to_string())
    }
}

impl From<&str> for LicenseObject {
    fn from(name: &str) -> Self {
        Self::new(name.to_string())
    }
}

impl ServerObject {
    /// Create a server object by url.
    pub fn new(url: impl Into<String>) -> ServerObject {
        Self {
            url: url.into(),
            description: None,
        }
    }

    /// Specifies an string describing the host designated by the URL.
    pub fn description(self, description: impl Into<String>) -> Self {
        Self {
            description: Some(description.into()),
            ..self
        }
    }
}

/// A license information for the exposed API.
pub struct LicenseObject {
    name: String,
    identifier: Option<String>,
    url: Option<String>,
}

impl From<String> for LicenseObject {
    fn from(name: String) -> Self {
        Self::new(name)
    }
}

impl LicenseObject {
    /// Create a license object by name.
    pub fn new(name: impl Into<String>) -> LicenseObject {
        Self {
            name: name.into(),
            identifier: None,
            url: None,
        }
    }

    /// Specifies an [`SPDX`](https://spdx.org/spdx-specification-21-web-version#h.jxpfx0ykyb60) license expression for the API.
    pub fn identifier(self, identifier: impl Into<String>) -> Self {
        Self {
            identifier: Some(identifier.into()),
            ..self
        }
    }

    /// Specifies a URL to the license used for the API.
    pub fn url(self, url: impl Into<String>) -> Self {
        Self {
            url: Some(url.into()),
            ..self
        }
    }
}

/// An OpenAPI service for Poem.
pub struct OpenApiService<T> {
    api: T,
    info: MetaInfo,
    servers: Vec<MetaServer>,
    cookie_key: Option<CookieKey>,
}

impl<T> OpenApiService<T> {
    /// Create an OpenAPI container.
    #[must_use]
    pub fn new(api: T, title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            api,
            info: MetaInfo {
                title: title.into(),
                description: None,
                version: version.into(),
                terms_of_service: None,
                license: None,
            },
            servers: Vec::new(),
            cookie_key: None,
        }
    }

    /// Sets the description of the API container.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.info.description = Some(description.into());
        self
    }

    /// Sets a URL to the Terms of Service for the API.
    pub fn terms_of_service(mut self, url: impl Into<String>) -> Self {
        self.info.terms_of_service = Some(url.into());
        self
    }

    /// Appends a server to the API container.
    ///
    /// Reference: <https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md#serverObject>
    #[must_use]
    pub fn server(mut self, server: impl Into<ServerObject>) -> Self {
        let server = server.into();
        self.servers.push(MetaServer {
            url: server.url,
            description: server.description,
        });
        self
    }

    /// Specifies the license information for the exposed API.
    ///
    /// Reference: https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md#license-object
    #[must_use]
    pub fn license(mut self, license: impl Into<LicenseObject>) -> Self {
        let license = license.into();
        self.info.license = Some(MetaLicense {
            name: license.name,
            identifier: license.identifier,
            url: license.url,
        });
        self
    }

    /// Sets the cookie key.
    pub fn cookie_key(self, key: CookieKey) -> Self {
        Self {
            cookie_key: Some(key),
            ..self
        }
    }

    /// Create the Swagger UI endpoint.
    #[must_use]
    #[cfg(feature = "swagger-ui")]
    pub fn swagger_ui(&self) -> impl Endpoint
    where
        T: OpenApi,
    {
        crate::ui::swagger_ui::create_endpoint(&self.spec())
    }

    /// Create the Rapidoc endpoint.
    #[must_use]
    #[cfg(feature = "rapidoc")]
    pub fn rapidoc(&self) -> impl Endpoint
    where
        T: OpenApi,
    {
        crate::ui::rapidoc::create_endpoint(&self.spec())
    }

    /// Create the Redoc endpoint.
    #[must_use]
    #[cfg(feature = "redoc")]
    pub fn redoc(&self) -> impl Endpoint
    where
        T: OpenApi,
    {
        crate::ui::redoc::create_endpoint(&self.spec())
    }

    /// Create an endpoint to serve the open api specification.
    pub fn spec_endpoint(&self) -> impl Endpoint
    where
        T: OpenApi,
    {
        let spec = self.spec();
        make_sync(move |_| {
            Response::builder()
                .content_type("application/json")
                .body(spec.clone())
        })
    }

    /// Returns the OAS specification file.
    pub fn spec(&self) -> String
    where
        T: OpenApi,
    {
        let mut registry = Registry::new();
        let metadata = T::meta();
        T::register(&mut registry);

        let doc = Document {
            info: &self.info,
            servers: &self.servers,
            apis: &metadata,
            registry: &registry,
        };
        serde_json::to_string_pretty(&doc).unwrap()
    }
}

impl<T: OpenApi> IntoEndpoint for OpenApiService<T> {
    type Endpoint = BoxEndpoint<'static, Response>;

    fn into_endpoint(self) -> Self::Endpoint {
        async fn extract_query(mut req: Request) -> Request {
            let url_query: Vec<(String, String)> = req.params().unwrap_or_default();
            req.extensions_mut().insert(UrlQuery(url_query));
            req
        }

        let cookie_jar_manager = match self.cookie_key {
            Some(key) => CookieJarManager::with_key(key),
            None => CookieJarManager::new(),
        };

        self.api
            .add_routes(Route::new())
            .with(cookie_jar_manager)
            .before(extract_query)
            .map_to_response()
            .boxed()
    }
}
