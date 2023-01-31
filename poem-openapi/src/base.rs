use std::{
    collections::HashMap,
    fmt::{self, Debug, Display},
    ops::Deref,
};

use poem::{endpoint::BoxEndpoint, http::Method, Error, FromRequest, Request, RequestBody, Result};

use crate::{
    payload::Payload,
    registry::{
        MetaApi, MetaMediaType, MetaOAuthScope, MetaParamIn, MetaRequest, MetaResponse,
        MetaResponses, MetaSchemaRef, MetaWebhook, Registry,
    },
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

    /// When this is `true`, parameter values of type array or object generate
    /// separate parameters for each value of the array or key-value pair of the
    /// map.
    pub explode: bool,
}

impl<T> Default for ExtractParamOptions<T> {
    fn default() -> Self {
        Self {
            name: "",
            default_value: None,
            explode: true,
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

/// Represents a OpenAPI response content object.
pub trait ResponseContent {
    /// Returns the media types in this content.
    fn media_types() -> Vec<MetaMediaType>;

    /// Register the schema contained in this content to the registry.
    #[allow(unused_variables)]
    fn register(registry: &mut Registry) {}
}

impl<T: Payload> ResponseContent for T {
    fn media_types() -> Vec<MetaMediaType> {
        vec![MetaMediaType {
            content_type: T::CONTENT_TYPE,
            schema: T::schema_ref(),
        }]
    }

    fn register(registry: &mut Registry) {
        T::register(registry);
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

impl ApiResponse for Error {
    fn meta() -> MetaResponses {
        MetaResponses {
            responses: Vec::new(),
        }
    }

    fn register(_registry: &mut Registry) {}
}

impl<T, E> ApiResponse for Result<T, E>
where
    T: ApiResponse,
    E: ApiResponse + Into<Error> + Send + Sync + 'static,
{
    const BAD_REQUEST_HANDLER: bool = T::BAD_REQUEST_HANDLER;

    fn meta() -> MetaResponses {
        let mut meta = T::meta();
        meta.responses.extend(E::meta().responses);
        meta
    }

    fn register(registry: &mut Registry) {
        T::register(registry);
        E::register(registry);
    }

    fn from_parse_request_error(err: Error) -> Self {
        Ok(T::from_parse_request_error(err))
    }
}

#[cfg(feature = "websocket")]
impl<F, Fut> ApiResponse for poem::web::websocket::WebSocketUpgraded<F>
where
    F: FnOnce(poem::web::websocket::WebSocketStream) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future + Send + 'static,
{
    fn meta() -> MetaResponses {
        MetaResponses {
            responses: vec![MetaResponse {
                description: "A websocket response",
                status: Some(101),
                content: vec![],
                headers: vec![],
            }],
        }
    }

    fn register(_registry: &mut Registry) {}
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

/// A operation id that can be obtained from the response
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct OperationId(pub &'static str);

impl Display for OperationId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// Represents a OpenAPI object.
pub trait OpenApi: Sized {
    /// Gets metadata of this API object.
    fn meta() -> Vec<MetaApi>;

    /// Register some types to the registry.
    fn register(registry: &mut Registry);

    /// Adds all API endpoints to the routing object.
    fn add_routes(self, route_table: &mut HashMap<String, HashMap<Method, BoxEndpoint<'static>>>);
}

macro_rules! impl_openapi_for_tuple {
    (($head:ident, $hn:tt), $(($tail:ident, $tn:tt)),*) => {
        impl<$head: OpenApi, $($tail: OpenApi),*> OpenApi for ($head, $($tail),*) {
            fn meta() -> Vec<MetaApi> {
                let mut metadata = $head::meta();
                $(
                metadata.extend($tail::meta());
                )*
                metadata
            }

            fn register(registry: &mut Registry) {
                $head::register(registry);
                $(
                $tail::register(registry);
                )*
            }

            fn add_routes(self, route_table: &mut HashMap<String, HashMap<Method, BoxEndpoint<'static>>>) {
                self.$hn.add_routes(route_table);
                $(
                self.$tn.add_routes(route_table);
                )*
            }
        }
    };

    () => {};
}

#[rustfmt::skip]
impl_openapi_for_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8), (T10, 9), (T11, 10), (T12, 11), (T13, 12), (T14, 13), (T15, 14), (T16, 15));
#[rustfmt::skip]
impl_openapi_for_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8), (T10, 9), (T11, 10), (T12, 11), (T13, 12), (T14, 13), (T15, 14));
#[rustfmt::skip]
impl_openapi_for_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8), (T10, 9), (T11, 10), (T12, 11), (T13, 12), (T14, 13));
#[rustfmt::skip]
impl_openapi_for_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8), (T10, 9), (T11, 10), (T12, 11), (T13, 12));
#[rustfmt::skip]
impl_openapi_for_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8), (T10, 9), (T11, 10), (T12, 11));
#[rustfmt::skip]
impl_openapi_for_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8), (T10, 9), (T11, 10));
#[rustfmt::skip]
impl_openapi_for_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8), (T10, 9));
#[rustfmt::skip]
impl_openapi_for_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8));
#[rustfmt::skip]
impl_openapi_for_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7));
#[rustfmt::skip]
impl_openapi_for_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6));
#[rustfmt::skip]
impl_openapi_for_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5));
#[rustfmt::skip]
impl_openapi_for_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4));
#[rustfmt::skip]
impl_openapi_for_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3));
#[rustfmt::skip]
impl_openapi_for_tuple!((T1, 0), (T2, 1), (T3, 2));
#[rustfmt::skip]
impl_openapi_for_tuple!((T1, 0), (T2, 1));

impl OpenApi for () {
    fn meta() -> Vec<MetaApi> {
        vec![]
    }

    fn register(_registry: &mut Registry) {}

    fn add_routes(self, _route_table: &mut HashMap<String, HashMap<Method, BoxEndpoint<'static>>>) {
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
