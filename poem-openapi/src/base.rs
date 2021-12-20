use std::ops::Deref;

use poem::{Error, FromRequest, Request, RequestBody, Result, Route};

use crate::registry::{
    MetaApi, MetaOAuthScope, MetaParamIn, MetaRequest, MetaResponse, MetaResponses, MetaSchemaRef,
    MetaWebhook, Registry,
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
///
/// # Provided Implementations
///
/// - **Path&lt;T: Type>**
///
///    Extract the parameters in the request path into
/// [`Path`](crate::param::Path).
///
/// - **Query&lt;T: Type>**
///
///    Extract the parameters in the query string into
/// [`Query`](crate::param::Query).
///
/// - **Header&lt;T: Type>**
///
///    Extract the parameters in the request header into
/// [`Header`](crate::param::Header).
///
/// - **Cookie&lt;T: Type>**
///
///    Extract the parameters in the cookie into
/// [`Cookie`](crate::param::Cookie).
///
/// - **CookiePrivate&lt;T: Type>**
///
///    Extract the parameters in the private cookie into
/// [`CookiePrivate`](crate::param::CookiePrivate).
///
/// - **CookieSigned&lt;T: Type>**
///
///    Extract the parameters in the signed cookie into
/// [`CookieSigned`](crate::param::CookieSigned).
///
/// - **Binary&lt;T>**
///
///     Extract the request body as binary into
/// [`Binary`](crate::payload::Binary).
///
/// - **Json&lt;T>**
///
///     Parse the request body in `JSON` format into
/// [`Json`](crate::payload::Json).
///
/// - **PlainText&lt;T>**
///
///     Extract the request body as utf8 string into
/// [`PlainText`](crate::payload::PlainText).
///
/// - **Any type derived from the [`ApiRequest`](crate::ApiRequest) macro**
///
///     Extract the complex request body derived from the `ApiRequest` macro.
///
/// - **Any type derived from the [`Multipart`](crate::Multipart) macro**
///
///     Extract the multipart object derived from the `Multipart` macro.
///
/// - **Any type derived from the [`SecurityScheme`](crate::SecurityScheme)
///   macro**
///
///     Extract the authentication value derived from the `SecurityScheme`
/// macro.
///
/// - **T: poem::FromRequest**
///
///     Use Poem's extractor.
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
/// # Provided Implementations
///
/// - **Binary&lt;T: Type>**
///
///     A binary response with content type `application/octet-stream`.
///
/// - **Json&lt;T: Type>**
///
///     A JSON response with content type `application/json`.
///
/// - **PlainText&lt;T: Type>**
///
///     A utf8 string response with content type `text/plain`.
///
/// - **Attachment&lt;T: Type>**
///
///     A file download response, the content type is
/// `application/octet-stream`.
///
/// - **Response&lt;T: Type>**
///
///     A response type use it to modify the status code and HTTP headers.
///
/// - **()**
///
///     It means that this API does not have any response body.
///
/// - **poem::Result&lt;T: ApiResponse>**
///
///     It means that an error may occur in this API.
///
/// - **Any type derived from the [`ApiResponse`](crate::ApiResponse) macro**
///
///     A complex response  derived from the `ApiResponse` macro.
pub trait ApiResponse: Sized {
    /// If true, it means that the response object has a custom bad request
    /// handler.
    const BAD_REQUEST_HANDLER: bool = false;

    /// Gets metadata of this response.
    fn meta() -> MetaResponses;

    /// Register the schema contained in this response object to the registry.
    fn register(registry: &mut Registry);

    /// Convert [`poem::Error`] to this response object.
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

impl OpenApi for () {
    fn meta() -> Vec<MetaApi> {
        vec![]
    }

    fn register(_registry: &mut Registry) {}

    fn add_routes(self, route: Route) -> Route {
        route
    }
}

/// Represents a webhook object.
pub trait Webhook: Sized {
    /// Gets metadata of this webhooks object.
    fn meta() -> Vec<MetaWebhook>;

    /// Register some types to the registry.
    fn register(registry: &mut Registry);
}

impl Webhook for () {
    fn meta() -> Vec<MetaWebhook> {
        vec![]
    }

    fn register(_: &mut Registry) {}
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
