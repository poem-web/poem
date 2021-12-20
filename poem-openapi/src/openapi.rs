use std::{collections::HashSet, marker::PhantomData};

use poem::{
    endpoint::{make_sync, BoxEndpoint},
    middleware::CookieJarManager,
    web::cookie::CookieKey,
    Endpoint, EndpointExt, IntoEndpoint, Request, Response, Result, Route,
};

use crate::{
    base::UrlQuery,
    registry::{Document, MetaExternalDocument, MetaInfo, MetaLicense, MetaServer, Registry},
    OpenApi, Webhook,
};

/// An object representing a Server.
#[derive(Debug, Clone)]
pub struct ServerObject {
    url: String,
    description: Option<String>,
}

impl<T: Into<String>> From<T> for ServerObject {
    fn from(url: T) -> Self {
        Self::new(url)
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

    /// Sets an string describing the host designated by the URL.
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

impl<T: Into<String>> From<T> for LicenseObject {
    fn from(url: T) -> Self {
        Self::new(url)
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

    /// Sets an [`SPDX`](https://spdx.org/spdx-specification-21-web-version#h.jxpfx0ykyb60) license expression for the API.
    pub fn identifier(self, identifier: impl Into<String>) -> Self {
        Self {
            identifier: Some(identifier.into()),
            ..self
        }
    }

    /// Sets a URL to the license used for the API.
    pub fn url(self, url: impl Into<String>) -> Self {
        Self {
            url: Some(url.into()),
            ..self
        }
    }
}

/// An object representing a external document.
#[derive(Debug, Clone)]
pub struct ExternalDocumentObject {
    url: String,
    description: Option<String>,
}

impl<T: Into<String>> From<T> for ExternalDocumentObject {
    fn from(url: T) -> Self {
        Self::new(url)
    }
}

impl ExternalDocumentObject {
    /// Create a external document object by url.
    pub fn new(url: impl Into<String>) -> ExternalDocumentObject {
        Self {
            url: url.into(),
            description: None,
        }
    }

    /// Sets a description of the target documentation..
    pub fn description(self, description: impl Into<String>) -> Self {
        Self {
            description: Some(description.into()),
            ..self
        }
    }
}

/// An OpenAPI service for Poem.
pub struct OpenApiService<T, W: ?Sized> {
    api: T,
    _webhook: PhantomData<W>,
    info: MetaInfo,
    external_document: Option<MetaExternalDocument>,
    servers: Vec<MetaServer>,
    cookie_key: Option<CookieKey>,
}

impl<T> OpenApiService<T, ()> {
    /// Create an OpenAPI container.
    #[must_use]
    pub fn new(api: T, title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            api,
            _webhook: PhantomData,
            info: MetaInfo {
                title: title.into(),
                description: None,
                version: version.into(),
                terms_of_service: None,
                license: None,
            },
            external_document: None,
            servers: Vec::new(),
            cookie_key: None,
        }
    }
}

impl<T, W: ?Sized> OpenApiService<T, W> {
    /// Sets the webhooks.
    pub fn webhooks<W2: ?Sized>(self) -> OpenApiService<T, W2> {
        OpenApiService {
            api: self.api,
            _webhook: PhantomData,
            info: self.info,
            external_document: self.external_document,
            servers: self.servers,
            cookie_key: self.cookie_key,
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

    /// Sets the license information for the exposed API.
    ///
    /// Reference: <https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md#license-object>
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

    /// Add a external document object.
    ///
    /// Reference: <https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md#external-documentation-object>
    #[must_use]
    pub fn external_document(
        mut self,
        external_document: impl Into<ExternalDocumentObject>,
    ) -> Self {
        let external_document = external_document.into();
        self.external_document = Some(MetaExternalDocument {
            url: external_document.url,
            description: external_document.description,
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
        W: Webhook,
    {
        crate::ui::swagger_ui::create_endpoint(&self.spec())
    }

    /// Create the Rapidoc endpoint.
    #[must_use]
    #[cfg(feature = "rapidoc")]
    pub fn rapidoc(&self) -> impl Endpoint
    where
        T: OpenApi,
        W: Webhook,
    {
        crate::ui::rapidoc::create_endpoint(&self.spec())
    }

    /// Create the Redoc endpoint.
    #[must_use]
    #[cfg(feature = "redoc")]
    pub fn redoc(&self) -> impl Endpoint
    where
        T: OpenApi,
        W: Webhook,
    {
        crate::ui::redoc::create_endpoint(&self.spec())
    }

    /// Create an endpoint to serve the open api specification.
    pub fn spec_endpoint(&self) -> impl Endpoint
    where
        T: OpenApi,
        W: Webhook,
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
        W: Webhook,
    {
        let mut registry = Registry::new();
        let metadata = T::meta();
        T::register(&mut registry);
        W::register(&mut registry);

        let webhooks = W::meta();

        let doc = Document {
            info: &self.info,
            servers: &self.servers,
            apis: &metadata,
            webhooks: &webhooks,
            registry: &registry,
            external_document: self.external_document.as_ref(),
        };
        serde_json::to_string_pretty(&doc).unwrap()
    }
}

impl<T: OpenApi, W: Webhook> IntoEndpoint for OpenApiService<T, W> {
    type Endpoint = BoxEndpoint<'static, Response>;

    fn into_endpoint(self) -> Self::Endpoint {
        async fn extract_query(mut req: Request) -> Result<Request> {
            let url_query: Vec<(String, String)> = req.params().unwrap_or_default();
            req.extensions_mut().insert(UrlQuery(url_query));
            Ok(req)
        }

        let cookie_jar_manager = match self.cookie_key {
            Some(key) => CookieJarManager::with_key(key),
            None => CookieJarManager::new(),
        };

        // check duplicate operation id
        let mut operation_ids = HashSet::new();
        for operation in T::meta()
            .into_iter()
            .map(|api| api.paths.into_iter())
            .flatten()
            .map(|path| path.operations.into_iter())
            .flatten()
        {
            if let Some(operation_id) = operation.operation_id {
                if !operation_ids.insert(operation_id) {
                    panic!("duplicate operation id: {}", operation_id);
                }
            }
        }

        self.api
            .add_routes(Route::new())
            .with(cookie_jar_manager)
            .before(extract_query)
            .map_to_response()
            .boxed()
    }
}
