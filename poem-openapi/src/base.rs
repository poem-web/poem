use std::collections::HashMap;

use poem::{route::Route, IntoResponse, Request, RequestBody, Result};

use crate::{
    payload::{ParsePayload, Payload},
    registry::{
        MetaApi, MetaMediaType, MetaOAuthScope, MetaRequest, MetaResponse, MetaResponses, Registry,
    },
    ParseRequestError,
};

/// Represents a OpenAPI request object.
///
/// Reference: <https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md#requestBodyObject>
#[poem::async_trait]
pub trait ApiRequest: Sized {
    /// Gets metadata of this request.
    fn meta() -> MetaRequest;

    /// Register the schema contained in this request object to the registry.
    fn register(registry: &mut Registry);

    /// Parse the request object from the HTTP request.
    async fn from_request(
        request: &Request,
        body: &mut RequestBody,
    ) -> Result<Self, ParseRequestError>;
}

#[poem::async_trait]
impl<T: Payload + ParsePayload> ApiRequest for T {
    fn meta() -> MetaRequest {
        MetaRequest {
            description: None,
            content: vec![MetaMediaType {
                content_type: T::CONTENT_TYPE,
                schema: T::schema_ref(),
            }],
            required: true,
        }
    }

    fn register(registry: &mut Registry) {
        T::register(registry);
    }

    async fn from_request(
        request: &Request,
        body: &mut RequestBody,
    ) -> Result<Self, ParseRequestError> {
        match request.content_type() {
            Some(content_type) if content_type != T::CONTENT_TYPE => {
                return Err(ParseRequestError::ContentTypeNotSupported {
                    content_type: content_type.to_string(),
                })
            }
            Some(_) => {}
            None => return Err(ParseRequestError::ExpectContentType),
        }

        <T as ParsePayload>::from_request(request, body).await
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
                description: None,
                status: Some(200),
                content: vec![],
                headers: vec![],
            }],
        }
    }

    fn register(_registry: &mut Registry) {}
}

impl<T: Payload + IntoResponse> ApiResponse for T {
    fn meta() -> MetaResponses {
        MetaResponses {
            responses: vec![MetaResponse {
                description: None,
                status: Some(200),
                content: vec![MetaMediaType {
                    content_type: T::CONTENT_TYPE,
                    schema: T::schema_ref(),
                }],
                headers: vec![],
            }],
        }
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

/// Represents a OpenAPI security scheme.
pub trait SecurityScheme: Sized {
    /// The name of security scheme.
    const NAME: &'static str;

    /// Register this security scheme type to registry.
    fn register(registry: &mut Registry);

    /// Parse authorization information from request.
    fn from_request(
        req: &Request,
        query: &HashMap<String, String>,
    ) -> Result<Self, ParseRequestError>;
}

/// Represents a OAuth scopes.
pub trait OAuthScopes {
    /// Gets metadata of this object.
    fn meta() -> Vec<MetaOAuthScope>;

    /// Get the scope name.
    fn name(&self) -> &'static str;
}

impl<T: SecurityScheme> SecurityScheme for Option<T> {
    const NAME: &'static str = T::NAME;

    fn register(registry: &mut Registry) {
        T::register(registry);
    }

    fn from_request(
        req: &Request,
        query: &HashMap<String, String>,
    ) -> Result<Self, ParseRequestError> {
        Ok(T::from_request(req, query).ok())
    }
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
