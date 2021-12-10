use poem::{
    endpoint::{make_sync, BoxEndpoint},
    middleware::CookieJarManager,
    web::cookie::CookieKey,
    Endpoint, EndpointExt, FromRequest, IntoEndpoint, IntoResponse, Request, Response, Route,
};

use crate::{
    base::UrlQuery,
    registry::{Document, MetaInfo, MetaServer, Registry},
    OpenApi,
};

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

    /// Appends a server to the API container.
    ///
    /// Reference: <https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md#serverObject>
    #[must_use]
    pub fn server(mut self, url: impl Into<String>) -> Self {
        self.servers.push(MetaServer {
            url: url.into(),
            description: None,
        });
        self
    }

    /// Appends a server and description to the API container.
    #[must_use]
    pub fn server_with_description(
        mut self,
        url: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        self.servers.push(MetaServer {
            url: url.into(),
            description: Some(description.into()),
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
        async fn extract_query(next: impl Endpoint, mut req: Request) -> impl IntoResponse {
            let query: poem::web::Query<Vec<(String, String)>> =
                FromRequest::from_request(&req, &mut Default::default())
                    .await
                    .unwrap_or_default();
            req.extensions_mut().insert(UrlQuery(query.0));
            next.call(req).await
        }

        match self.cookie_key {
            Some(key) => self
                .api
                .add_routes(Route::new())
                .with(CookieJarManager::with_key(key))
                .around(extract_query)
                .map_to_response()
                .boxed(),
            None => self
                .api
                .add_routes(Route::new())
                .with(CookieJarManager::new())
                .around(extract_query)
                .map_to_response()
                .boxed(),
        }
    }
}
