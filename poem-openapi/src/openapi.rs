use poem::{
    endpoint::{make_sync, BoxEndpoint},
    web::cookie::CookieKey,
    Endpoint, EndpointExt, IntoEndpoint, Response, Route,
};

#[cfg(feature = "swagger-ui")]
use crate::ui::create_ui_endpoint;
use crate::{
    poem::middleware::CookieJarManager,
    registry::{Document, MetaInfo, MetaServer, Registry},
    OpenApi,
};

/// An OpenAPI service for Poem.
pub struct OpenApiService<T> {
    api: T,
    info: Option<MetaInfo>,
    servers: Vec<MetaServer>,
    cookie_key: Option<CookieKey>,
}

impl<T> OpenApiService<T> {
    /// Create an OpenAPI container.
    #[must_use]
    pub fn new(api: T) -> Self {
        Self {
            api,
            info: None,
            servers: Vec::new(),
            cookie_key: None,
        }
    }

    /// Sets the title of the API container.
    ///
    /// Reference: <https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md#infoObject>
    #[must_use]
    pub fn title(mut self, title: &'static str) -> Self {
        self.info.get_or_insert_with(Default::default).title = Some(title);
        self
    }

    /// Sets the description of the API container.
    #[must_use]
    pub fn description(mut self, description: &'static str) -> Self {
        self.info.get_or_insert_with(Default::default).description = Some(description);
        self
    }

    /// Sets the version of the API container.
    ///
    /// NOTE: The version of the OpenAPI document (which is distinct from the
    /// OpenAPI Specification version or the API implementation version).
    ///
    /// Reference: <https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md#infoObject>
    #[must_use]
    pub fn version(mut self, version: &'static str) -> Self {
        self.info.get_or_insert_with(Default::default).version = Some(version);
        self
    }

    /// Appends a server to the API container.
    ///
    /// Reference: <https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md#serverObject>
    #[must_use]
    pub fn server(mut self, url: &'static str) -> Self {
        self.servers.push(MetaServer {
            url,
            description: None,
        });
        self
    }

    /// Appends a server and description to the API container.
    #[must_use]
    pub fn server_with_description(mut self, url: &'static str, description: &'static str) -> Self {
        self.servers.push(MetaServer {
            url,
            description: Some(description),
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
        create_ui_endpoint(&self.spec())
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
            info: self.info.as_ref(),
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
        match self.cookie_key {
            Some(key) => self
                .api
                .add_routes(Route::new())
                .with(CookieJarManager::with_key(key))
                .boxed(),
            None => self
                .api
                .add_routes(Route::new())
                .with(CookieJarManager::new())
                .boxed(),
        }
    }
}
