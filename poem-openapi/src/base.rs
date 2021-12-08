use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

use mime::Mime;
use poem::{FromRequest, IntoResponse, Request, RequestBody, Result, Route};

use crate::{
    payload::{ParsePayload, Payload},
    registry::{
        MetaApi, MetaMediaType, MetaOAuthScope, MetaParamIn, MetaRequest, MetaResponse,
        MetaResponses, MetaSchemaRef, Registry,
    },
    ParseRequestError,
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
pub struct UrlQuery(pub BTreeMap<String, String>);

impl Deref for UrlQuery {
    type Target = BTreeMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
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
    ) -> Result<Self, ParseRequestError>;
}

#[poem::async_trait]
impl<'a, T: Payload + ParsePayload> ApiExtractor<'a> for T {
    const TYPE: ApiExtractorType = ApiExtractorType::RequestObject;

    type ParamType = ();
    type ParamRawType = ();

    fn register(registry: &mut Registry) {
        T::register(registry);
    }

    fn request_meta() -> Option<MetaRequest> {
        Some(MetaRequest {
            description: None,
            content: vec![MetaMediaType {
                content_type: T::CONTENT_TYPE,
                schema: T::schema_ref(),
            }],
            required: T::IS_REQUIRED,
        })
    }

    async fn from_request(
        request: &'a Request,
        body: &mut RequestBody,
        _param_opts: ExtractParamOptions<Self::ParamType>,
    ) -> Result<Self, ParseRequestError> {
        match request.content_type() {
            Some(content_type) => {
                let mime: Mime = match content_type.parse() {
                    Ok(mime) => mime,
                    Err(_) => {
                        return Err(ParseRequestError::ContentTypeNotSupported {
                            content_type: content_type.to_string(),
                        });
                    }
                };

                if mime.essence_str() != T::CONTENT_TYPE {
                    return Err(ParseRequestError::ContentTypeNotSupported {
                        content_type: content_type.to_string(),
                    });
                }

                <T as ParsePayload>::from_request(request, body).await
            }
            None => Err(ParseRequestError::ExpectContentType),
        }
    }
}

#[poem::async_trait]
impl<'a, T: ApiExtractor<'a>> ApiExtractor<'a> for Result<T, ParseRequestError> {
    const TYPE: ApiExtractorType = T::TYPE;
    const PARAM_IS_REQUIRED: bool = T::PARAM_IS_REQUIRED;
    type ParamType = T::ParamType;
    type ParamRawType = T::ParamRawType;

    fn register(registry: &mut Registry) {
        T::register(registry);
    }

    fn security_scheme() -> Option<&'static str> {
        T::security_scheme()
    }

    fn param_in() -> Option<MetaParamIn> {
        T::param_in()
    }

    fn param_schema_ref() -> Option<MetaSchemaRef> {
        T::param_schema_ref()
    }

    fn request_meta() -> Option<MetaRequest> {
        T::request_meta()
    }

    fn param_raw_type(&self) -> Option<&Self::ParamRawType> {
        match self {
            Ok(value) => value.param_raw_type(),
            Err(_) => None,
        }
    }

    async fn from_request(
        request: &'a Request,
        body: &mut RequestBody,
        param_opts: ExtractParamOptions<Self::ParamType>,
    ) -> Result<Self, ParseRequestError> {
        Ok(T::from_request(request, body, param_opts).await)
    }
}

/// Represents a poem extractor.
pub struct PoemExtractor<T>(pub T);

impl<T> Deref for PoemExtractor<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for PoemExtractor<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[poem::async_trait]
impl<'a, T: FromRequest<'a>> ApiExtractor<'a> for PoemExtractor<T> {
    const TYPE: ApiExtractorType = ApiExtractorType::PoemExtractor;

    type ParamType = ();
    type ParamRawType = ();

    async fn from_request(
        request: &'a Request,
        body: &mut RequestBody,
        _param_opts: ExtractParamOptions<Self::ParamType>,
    ) -> Result<Self, ParseRequestError> {
        match T::from_request(request, body).await {
            Ok(value) => Ok(Self(value)),
            Err(err) => Err(ParseRequestError::Extractor(err.into_response())),
        }
    }
}

/// Represents a OpenAPI responses object.
///
/// Reference: <https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md#responsesObject>
pub trait ApiResponse: IntoResponse + Sized {
    /// If true, it means that the response object has a custom bad request
    /// handler.
    const BAD_REQUEST_HANDLER: bool = false;

    /// Gets metadata of this response.
    fn meta() -> MetaResponses;

    /// Register the schema contained in this response object to the registry.
    fn register(registry: &mut Registry);

    /// Convert [`ParseRequestError`] to this response object.
    #[allow(unused_variables)]
    fn from_parse_request_error(err: ParseRequestError) -> Self {
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

impl<T: ApiResponse, E: IntoResponse> ApiResponse for Result<T, E> {
    fn meta() -> MetaResponses {
        T::meta()
    }

    fn register(registry: &mut Registry) {
        T::register(registry);
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
