use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use poem::{
    endpoint::{make_sync, BoxEndpoint},
    middleware::CookieJarManager,
    web::cookie::CookieKey,
    Endpoint, EndpointExt, IntoEndpoint, Request, Response, Result, Route, RouteMethod,
};

use crate::{
    base::UrlQuery,
    registry::{
        Document, MetaContact, MetaExternalDocument, MetaHeader, MetaInfo, MetaLicense,
        MetaOperationParam, MetaParamIn, MetaSchemaRef, MetaServer, Registry,
    },
    types::Type,
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
    #[must_use]
    pub fn description(self, description: impl Into<String>) -> Self {
        Self {
            description: Some(description.into()),
            ..self
        }
    }
}

/// A contact information for the exposed API.
#[derive(Debug, Default)]
pub struct ContactObject {
    name: Option<String>,
    url: Option<String>,
    email: Option<String>,
}

impl ContactObject {
    /// Create a new Contact object
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the identifying name of the contact person/organization.
    #[must_use]
    pub fn name(self, name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ..self
        }
    }

    /// Sets the URL pointing to the contact information.
    #[must_use]
    pub fn url(self, url: impl Into<String>) -> Self {
        Self {
            url: Some(url.into()),
            ..self
        }
    }

    /// Sets the email address of the contact person/organization.
    #[must_use]
    pub fn email(self, email: impl Into<String>) -> Self {
        Self {
            email: Some(email.into()),
            ..self
        }
    }
}

/// A license information for the exposed API.
#[derive(Debug)]
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

    /// Sets the [`SPDX`](https://spdx.org/spdx-specification-21-web-version#h.jxpfx0ykyb60) license expression for the API.
    #[must_use]
    pub fn identifier(self, identifier: impl Into<String>) -> Self {
        Self {
            identifier: Some(identifier.into()),
            ..self
        }
    }

    /// Sets the URL to the license used for the API.
    #[must_use]
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

    /// Sets a description of the target documentation.
    #[must_use]
    pub fn description(self, description: impl Into<String>) -> Self {
        Self {
            description: Some(description.into()),
            ..self
        }
    }
}

/// An extra header
pub struct ExtraHeader {
    name: String,
    description: Option<String>,
    deprecated: bool,
}

impl<T: AsRef<str>> From<T> for ExtraHeader {
    fn from(name: T) -> Self {
        Self::new(name)
    }
}

impl ExtraHeader {
    /// Create a extra header object by name.
    pub fn new(name: impl AsRef<str>) -> ExtraHeader {
        Self {
            name: name.as_ref().to_uppercase(),
            description: None,
            deprecated: false,
        }
    }

    /// Sets a description of the extra header.
    #[must_use]
    pub fn description(self, description: impl Into<String>) -> Self {
        Self {
            description: Some(description.into()),
            ..self
        }
    }

    /// Specifies this header is deprecated.
    pub fn deprecated(self) -> Self {
        Self {
            deprecated: true,
            ..self
        }
    }
}

/// An OpenAPI service for Poem.
pub struct OpenApiService<T, W> {
    api: T,
    _webhook: PhantomData<W>,
    info: MetaInfo,
    external_document: Option<MetaExternalDocument>,
    servers: Vec<MetaServer>,
    cookie_key: Option<CookieKey>,
    extra_response_headers: Vec<(ExtraHeader, MetaSchemaRef, bool)>,
    extra_request_headers: Vec<(ExtraHeader, MetaSchemaRef, bool)>,
    url_prefix: Option<String>,
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
                summary: None,
                description: None,
                version: version.into(),
                terms_of_service: None,
                contact: None,
                license: None,
            },
            external_document: None,
            servers: Vec::new(),
            cookie_key: None,
            extra_response_headers: vec![],
            extra_request_headers: vec![],
            url_prefix: None,
        }
    }
}

impl<T, W> OpenApiService<T, W> {
    /// Sets the webhooks.
    pub fn webhooks<W2>(self) -> OpenApiService<T, W2> {
        OpenApiService {
            api: self.api,
            _webhook: PhantomData,
            info: self.info,
            external_document: self.external_document,
            servers: self.servers,
            cookie_key: self.cookie_key,
            extra_response_headers: self.extra_response_headers,
            extra_request_headers: self.extra_request_headers,
            url_prefix: None,
        }
    }

    /// Sets the summary of the API container.
    #[must_use]
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.info.summary = Some(summary.into());
        self
    }

    /// Sets the description of the API container.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.info.description = Some(description.into());
        self
    }

    /// Sets a URL to the Terms of Service for the API.
    #[must_use]
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

    /// Sets the contact information for the exposed API.
    #[must_use]
    pub fn contact(mut self, contact: ContactObject) -> Self {
        self.info.contact = Some(MetaContact {
            name: contact.name,
            url: contact.url,
            email: contact.email,
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

    /// Add extra response header
    #[must_use]
    pub fn extra_response_header<HT, H>(mut self, header: H) -> Self
    where
        HT: Type,
        H: Into<ExtraHeader>,
    {
        let extra_header = header.into();
        self.extra_response_headers
            .push((extra_header, HT::schema_ref(), HT::IS_REQUIRED));
        self
    }

    /// Add extra request header
    #[must_use]
    pub fn extra_request_header<HT, H>(mut self, header: H) -> Self
    where
        HT: Type,
        H: Into<ExtraHeader>,
    {
        let extra_header = header.into();
        self.extra_request_headers
            .push((extra_header, HT::schema_ref(), HT::IS_REQUIRED));
        self
    }

    /// Sets the cookie key.
    #[must_use]
    pub fn cookie_key(self, key: CookieKey) -> Self {
        Self {
            cookie_key: Some(key),
            ..self
        }
    }

    /// Sets optional URl prefix to be added to path
    pub fn url_prefix(self, url_prefix: impl Into<String>) -> Self {
        Self {
            url_prefix: Some(url_prefix.into()),
            ..self
        }
    }

    /// Create the OpenAPI Explorer endpoint.
    #[must_use]
    #[cfg(feature = "openapi-explorer")]
    pub fn openapi_explorer(&self) -> impl Endpoint
    where
        T: OpenApi,
        W: Webhook,
    {
        crate::ui::openapi_explorer::create_endpoint(&self.spec())
    }

    /// Create the OpenAPI Explorer HTML
    #[cfg(feature = "openapi-explorer")]
    pub fn openapi_explorer_html(&self) -> String
    where
        T: OpenApi,
        W: Webhook,
    {
        crate::ui::openapi_explorer::create_html(&self.spec())
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

    /// Create the Swagger UI HTML
    #[cfg(feature = "swagger-ui")]
    pub fn swagger_ui_html(&self) -> String
    where
        T: OpenApi,
        W: Webhook,
    {
        crate::ui::swagger_ui::create_html(&self.spec())
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

    /// Create the Rapidoc HTML
    #[cfg(feature = "rapidoc")]
    pub fn rapidoc_html(&self) -> String
    where
        T: OpenApi,
        W: Webhook,
    {
        crate::ui::rapidoc::create_html(&self.spec())
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

    /// Create the Redoc HTML
    #[must_use]
    #[cfg(feature = "redoc")]
    pub fn redoc_html(&self) -> String
    where
        T: OpenApi,
        W: Webhook,
    {
        crate::ui::redoc::create_html(&self.spec())
    }

    /// Create an endpoint to serve the open api specification as JSON.
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

    /// Create an endpoint to serve the open api specification as YAML.
    pub fn spec_endpoint_yaml(&self) -> impl Endpoint
    where
        T: OpenApi,
        W: Webhook,
    {
        let spec = self.spec_yaml();
        make_sync(move |_| {
            Response::builder()
                .content_type("application/x-yaml")
                .header("Content-Disposition", "inline; filename=\"spec.yaml\"")
                .body(spec.clone())
        })
    }

    fn document(&self) -> Document<'_>
    where
        T: OpenApi,
        W: Webhook,
    {
        let mut registry = Registry::new();
        let mut apis = T::meta();

        // update extra request headers
        for operation in apis
            .iter_mut()
            .flat_map(|meta_api| meta_api.paths.iter_mut())
            .flat_map(|path| path.operations.iter_mut())
        {
            for (idx, (header, schema_ref, is_required)) in
                self.extra_request_headers.iter().enumerate()
            {
                operation.params.insert(
                    idx,
                    MetaOperationParam {
                        name: header.name.clone(),
                        schema: schema_ref.clone(),
                        in_type: MetaParamIn::Header,
                        description: header.description.clone(),
                        required: *is_required,
                        deprecated: header.deprecated,
                        explode: true,
                    },
                );
            }
        }

        // update extra response headers
        for resp in apis
            .iter_mut()
            .flat_map(|meta_api| meta_api.paths.iter_mut())
            .flat_map(|path| path.operations.iter_mut())
            .flat_map(|operation| operation.responses.responses.iter_mut())
        {
            for (idx, (header, schema_ref, is_required)) in
                self.extra_response_headers.iter().enumerate()
            {
                resp.headers.insert(
                    idx,
                    MetaHeader {
                        name: header.name.clone(),
                        description: header.description.clone(),
                        required: *is_required,
                        deprecated: header.deprecated,
                        schema: schema_ref.clone(),
                    },
                );
            }
        }

        T::register(&mut registry);
        W::register(&mut registry);

        let webhooks = W::meta();

        let mut doc = Document {
            info: &self.info,
            servers: &self.servers,
            apis,
            webhooks,
            registry,
            external_document: self.external_document.as_ref(),
            url_prefix: self.url_prefix.as_deref(),
        };
        doc.remove_unused_schemas();

        doc
    }

    /// Returns the OAS specification file as JSON.
    pub fn spec(&self) -> String
    where
        T: OpenApi,
        W: Webhook,
    {
        let doc = self.document();
        serde_json::to_string_pretty(&doc).unwrap()
    }

    /// Returns the OAS specification file as YAML.
    pub fn spec_yaml(&self) -> String
    where
        T: OpenApi,
        W: Webhook,
    {
        let doc = self.document();
        serde_yaml::to_string(&doc).unwrap()
    }
}

impl<T: OpenApi, W: Webhook> IntoEndpoint for OpenApiService<T, W> {
    type Endpoint = BoxEndpoint<'static>;

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
            .flat_map(|api| api.paths.into_iter())
            .flat_map(|path| path.operations.into_iter())
        {
            if let Some(operation_id) = operation.operation_id {
                if !operation_ids.insert(operation_id) {
                    panic!("duplicate operation id: {operation_id}");
                }
            }
        }

        let mut items = HashMap::new();
        self.api.add_routes(&mut items);

        let route = items
            .into_iter()
            .fold(Route::new(), |route, (path, paths)| {
                route.at(
                    path,
                    paths
                        .into_iter()
                        .fold(RouteMethod::new(), |route_method, (method, ep)| {
                            route_method.method(method, ep)
                        }),
                )
            });

        route
            .with(cookie_jar_manager)
            .before(extract_query)
            .map_to_response()
            .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{types::Type, OpenApi};

    #[test]
    fn extra_response_headers() {
        struct Api;

        #[OpenApi(internal)]
        impl Api {
            #[oai(path = "/", method = "get")]
            async fn test(&self) {}
        }

        let api_service = OpenApiService::new(Api, "demo", "1.0")
            .extra_response_header::<i32, _>("a1")
            .extra_response_header::<String, _>(ExtraHeader::new("A2").description("abc"))
            .extra_response_header::<f32, _>(ExtraHeader::new("A3").deprecated());
        let doc = api_service.document();
        let headers = &doc.apis[0].paths[0].operations[0].responses.responses[0].headers;

        assert_eq!(headers[0].name, "A1");
        assert_eq!(headers[0].description, None);
        assert!(!headers[0].deprecated);
        assert_eq!(headers[0].schema, i32::schema_ref());

        assert_eq!(headers[1].name, "A2");
        assert_eq!(headers[1].description.as_deref(), Some("abc"));
        assert!(!headers[1].deprecated);
        assert_eq!(headers[1].schema, String::schema_ref());

        assert_eq!(headers[2].name, "A3");
        assert_eq!(headers[2].description, None);
        assert!(headers[2].deprecated);
        assert_eq!(headers[2].schema, f32::schema_ref());
    }

    #[test]
    fn extra_request_headers() {
        struct Api;

        #[OpenApi(internal)]
        impl Api {
            #[oai(path = "/", method = "get")]
            async fn test(&self) {}
        }

        let api_service = OpenApiService::new(Api, "demo", "1.0")
            .extra_request_header::<i32, _>("a1")
            .extra_request_header::<String, _>(ExtraHeader::new("A2").description("abc"))
            .extra_request_header::<f32, _>(ExtraHeader::new("A3").deprecated());
        let doc = api_service.document();
        let params = &doc.apis[0].paths[0].operations[0].params;

        assert_eq!(params[0].name, "A1");
        assert_eq!(params[0].in_type, MetaParamIn::Header);
        assert_eq!(params[0].description, None);
        assert!(!params[0].deprecated);
        assert_eq!(params[0].schema, i32::schema_ref());

        assert_eq!(params[1].name, "A2");
        assert_eq!(params[1].in_type, MetaParamIn::Header);
        assert_eq!(params[1].description.as_deref(), Some("abc"));
        assert!(!params[1].deprecated);
        assert_eq!(params[1].schema, String::schema_ref());

        assert_eq!(params[2].name, "A3");
        assert_eq!(params[2].in_type, MetaParamIn::Header);
        assert_eq!(params[2].description, None);
        assert!(params[2].deprecated);
        assert_eq!(params[2].schema, f32::schema_ref());
    }
}
