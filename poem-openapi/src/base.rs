use std::ops::Deref;

use poem::{Error, FromRequest, Request, RequestBody, Result, Route};

use crate::registry::{
    MetaApi, MetaOAuthScope, MetaParamIn, MetaRequest, MetaResponse, MetaResponses, MetaSchemaRef,
    Registry,
};

/// API extractor types.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ApiExtractorType {
    /// A request object.
    RequestObject,

    /// A request parameter.
    Parameter,

    /// A security scheme.
    SecurityScheme,

    /// A poem extractor.
    PoemExtractor,
}

#[doc(hidden)]
pub struct UrlQuery(pub Vec<(String, String)>);

impl Deref for UrlQuery {
    type Target = Vec<(String, String)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UrlQuery {
    #[allow(missing_docs)]
    pub fn get_all<'a, 'b: 'a>(&'b self, name: &'a str) -> impl Iterator<Item = &'b String> + 'a {
        self.0
            .iter()
            .filter(move |(n, _)| n == name)
            .map(|(_, value)| value)
    }

    #[allow(missing_docs)]
    pub fn get(&self, name: &str) -> Option<&String> {
        self.get_all(name).next()
    }
}

/// Options for the parameter extractor.
pub struct ExtractParamOptions<T> {
    /// The name of this parameter.
    pub name: &'static str,

    /// The default value of this parameter.
    pub default_value: Option<fn() -> T>,
}

impl<T> Default for ExtractParamOptions<T> {
    fn default() -> Self {
        Self {
            name: "",
            default_value: None,
        }
    }
}

/// Represents a OpenAPI extractor.
#[poem::async_trait]
#[allow(unused_variables)]
pub trait ApiExtractor<'a>: Sized {
    /// The type of API extractor.
    const TYPE: ApiExtractorType;

    /// If it is `true`, it means that this parameter is required.
    const PARAM_IS_REQUIRED: bool = false;

    /// The parameter type.
    type ParamType;

    /// The raw parameter type for validators.
    type ParamRawType;

    /// Register related types to registry.
    fn register(registry: &mut Registry) {}

    /// Returns name of security scheme if this extractor is security scheme.
    fn security_scheme() -> Option<&'static str> {
        None
    }

    /// Returns the location of the parameter if this extractor is parameter.
    fn param_in() -> Option<MetaParamIn> {
        None
    }

    /// Returns the schema of the parameter if this extractor is parameter.
    fn param_schema_ref() -> Option<MetaSchemaRef> {
        None
    }

    /// Returns `MetaRequest` if this extractor is request object.
    fn request_meta() -> Option<MetaRequest> {
        None
    }

    /// Returns a reference to the raw type of this parameter.
    fn param_raw_type(&self) -> Option<&Self::ParamRawType> {
        None
    }

    /// Parse from the HTTP request.
    async fn from_request(
        request: &'a Request,
        body: &mut RequestBody,
        param_opts: ExtractParamOptions<Self::ParamType>,
    ) -> Result<Self>;
}

#[poem::async_trait]
impl<'a, T: FromRequest<'a>> ApiExtractor<'a> for T {
    const TYPE: ApiExtractorType = ApiExtractorType::PoemExtractor;

    type ParamType = ();
    type ParamRawType = ();

    async fn from_request(
        request: &'a Request,
        body: &mut RequestBody,
        _param_opts: ExtractParamOptions<Self::ParamType>,
    ) -> Result<Self> {
        T::from_request(request, body).await
    }
}

/// Represents a OpenAPI responses object.
///
/// Reference: <https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md#responsesObject>
pub trait ApiResponse: Sized {
    /// If true, it means that the response object has a custom bad request
    /// handler.
    const BAD_REQUEST_HANDLER: bool = false;

    /// Gets metadata of this response.
    fn meta() -> MetaResponses;

    /// Register the schema contained in this response object to the registry.
    fn register(registry: &mut Registry);

    /// Convert [`ParseRequestError`] to this response object.
    #[allow(unused_variables)]
    fn from_parse_request_error(err: Error) -> Self {
        unreachable!()
    }
}

impl ApiResponse for () {
    fn meta() -> MetaResponses {
        MetaResponses {
            responses: vec![MetaResponse {
                description: "",
                status: Some(200),
                content: vec![],
                headers: vec![],
            }],
        }
    }

    fn register(_registry: &mut Registry) {}
}

impl<T: ApiResponse> ApiResponse for Result<T> {
    const BAD_REQUEST_HANDLER: bool = T::BAD_REQUEST_HANDLER;

    fn meta() -> MetaResponses {
        T::meta()
    }

    fn register(registry: &mut Registry) {
        T::register(registry);
    }

    fn from_parse_request_error(err: Error) -> Self {
        Ok(T::from_parse_request_error(err))
    }
}

/// Represents a OpenAPI tags.
pub trait Tags {
    /// Register this tag type to registry.
    fn register(&self, registry: &mut Registry);

    /// Gets the tag name.
    fn name(&self) -> &'static str;
}

/// Represents a OAuth scopes.
pub trait OAuthScopes {
    /// Gets metadata of this object.
    fn meta() -> Vec<MetaOAuthScope>;

    /// Get the scope name.
    fn name(&self) -> &'static str;
}

/// Represents a OpenAPI object.
pub trait OpenApi: Sized {
    /// Gets metadata of this API object.
    fn meta() -> Vec<MetaApi>;

    /// Register some types to the registry.
    fn register(registry: &mut Registry);

    /// Adds all API endpoints to the routing object.
    fn add_routes(self, route: Route) -> Route;

    /// Combine two API objects into one.
    fn combine<T: OpenApi>(self, other: T) -> CombinedAPI<Self, T> {
        CombinedAPI(self, other)
    }
}

/// API for the [`combine`](crate::OpenApi::combine) method.
pub struct CombinedAPI<A, B>(A, B);

impl<A: OpenApi, B: OpenApi> OpenApi for CombinedAPI<A, B> {
    fn meta() -> Vec<MetaApi> {
        let mut metadata = A::meta();
        metadata.extend(B::meta());
        metadata
    }

    fn register(registry: &mut Registry) {
        A::register(registry);
        B::register(registry);
    }

    fn add_routes(self, route: Route) -> Route {
        self.1.add_routes(self.0.add_routes(route))
    }
}
