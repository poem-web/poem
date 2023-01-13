use poem::{web::StaticFileResponse, Body};

use crate::{
    payload::{Binary, Payload},
    registry::{MetaHeader, MetaMediaType, MetaResponse, MetaResponses, Registry},
    types::Type,
    ApiResponse,
};

const ETAG_DESCRIPTION: &str = r#"The ETag (or entity tag) HTTP response header is an identifier for a specific version of a resource. It lets caches be more efficient and save bandwidth, as a web server does not need to resend a full response if the content was not changed. Additionally, etags help to prevent simultaneous updates of a resource from overwriting each other ("mid-air collisions")."#;
const LAST_MODIFIED_DESCRIPTION: &str = r#"The Last-Modified response HTTP header contains a date and time when the origin server believes the resource was last modified. It is used as a validator to determine if the resource is the same as the previously stored one. Less accurate than an ETag header, it is a fallback mechanism. Conditional requests containing If-Modified-Since or If-Unmodified-Since headers make use of this field."#;
const CONTENT_TYPE_DESCRIPTION: &str = r#"The Content-Type representation header is used to indicate the original media type of the resource (prior to any content encoding applied for sending)."#;

impl ApiResponse for StaticFileResponse {
    fn meta() -> MetaResponses {
        MetaResponses {
            responses: vec![
                MetaResponse {
                    description: "",
                    status: Some(200),
                    content: vec![MetaMediaType {
                        content_type: Binary::<Body>::CONTENT_TYPE,
                        schema: Binary::<Body>::schema_ref(),
                    }],
                    headers: vec![MetaHeader {
                        name: "etag".to_string(),
                        description: Some(ETAG_DESCRIPTION.to_string()),
                        required: false,
                        deprecated: false,
                        schema: String::schema_ref(),
                    }, MetaHeader {
                        name: "last-modified".to_string(),
                        description: Some(LAST_MODIFIED_DESCRIPTION.to_string()),
                        required: false,
                        deprecated: false,
                        schema: String::schema_ref(),
                    }, MetaHeader {
                        name: "content-type".to_string(),
                        description: Some(CONTENT_TYPE_DESCRIPTION.to_string()),
                        required: false,
                        deprecated: false,
                        schema: String::schema_ref(),
                    }],
                },
                MetaResponse {
                    description: "Not modified",
                    status: Some(304),
                    content: vec![],
                    headers: vec![],
                },
                MetaResponse {
                    description: "Bad request",
                    status: Some(400),
                    content: vec![],
                    headers: vec![],
                },
                MetaResponse {
                    description: "Resource was not found",
                    status: Some(404),
                    content: vec![],
                    headers: vec![],
                },
                MetaResponse {
                    description: "Precondition failed",
                    status: Some(412),
                    content: vec![],
                    headers: vec![],
                },
                MetaResponse {
                    description: "The Content-Range response HTTP header indicates where in a full body message a partial message belongs.",
                    status: Some(416),
                    content: vec![],
                    headers: vec![],
                }, MetaResponse {
                    description: "Internal server error",
                    status: Some(500),
                    content: vec![],
                    headers: vec![],
                },
            ],
        }
    }

    fn register(_registry: &mut Registry) {}
}
