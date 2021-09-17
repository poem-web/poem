mod request;

use poem::{
    http::{HeaderValue, StatusCode},
    IntoResponse,
};
use poem_openapi::{
    payload::{Json, PlainText},
    registry::{MetaHeader, MetaMediaType, MetaResponse, MetaResponses, MetaSchema, MetaSchemaRef},
    ApiResponse, Object,
};
use serde_json::Value;

#[derive(Object)]
struct BadRequestResult {
    error_code: i32,
    message: String,
}

#[derive(ApiResponse)]
enum MyResponse {
    /// Ok
    #[oai(status = 200)]
    Ok,
    /// A
    /// B
    ///
    /// C
    #[oai(status = 400)]
    BadRequest(Json<BadRequestResult>),
    Default(StatusCode, PlainText<String>),
}

#[test]
fn meta() {
    assert_eq!(
        MyResponse::meta(),
        MetaResponses {
            responses: vec![
                MetaResponse {
                    description: Some("Ok"),
                    status: Some(200),
                    content: vec![],
                    headers: vec![]
                },
                MetaResponse {
                    description: Some("A\nB\n\nC"),
                    status: Some(400),
                    content: vec![MetaMediaType {
                        content_type: "application/json",
                        schema: MetaSchemaRef::Reference("BadRequestResult")
                    }],
                    headers: vec![]
                },
                MetaResponse {
                    description: None,
                    status: None,
                    content: vec![MetaMediaType {
                        content_type: "text/plain",
                        schema: MetaSchemaRef::Inline(MetaSchema::new("string")),
                    }],
                    headers: vec![]
                }
            ],
        },
    );
}

#[tokio::test]
async fn into_response() {
    let resp = MyResponse::Ok.into_response();
    assert_eq!(resp.status(), StatusCode::OK);

    let mut resp = MyResponse::BadRequest(Json(BadRequestResult {
        error_code: 123,
        message: "abc".to_string(),
    }))
    .into_response();
    assert_eq!(
        serde_json::from_slice::<Value>(&resp.take_body().into_bytes().await.unwrap()).unwrap(),
        serde_json::json!({
            "errorCode": 123,
            "message": "abc",
        })
    );
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let mut resp = MyResponse::Default(StatusCode::BAD_GATEWAY, PlainText("abcdef".to_string()))
        .into_response();
    assert_eq!(resp.take_body().into_string().await.unwrap(), "abcdef");
    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn headers() {
    #[derive(ApiResponse)]
    enum MyResponse {
        #[oai(status = 200)]
        A,
        #[oai(status = 200)]
        B(
            #[oai(header = "MY-HEADER1", desc = "header1")] i32,
            #[oai(header = "MY-HEADER2")] String,
        ),
        #[oai(status = 400)]
        C(
            Json<BadRequestResult>,
            #[oai(header = "MY-HEADER1")] i32,
            #[oai(header = "MY-HEADER2")] String,
        ),
        D(
            StatusCode,
            PlainText<String>,
            #[oai(header = "MY-HEADER1")] i32,
            #[oai(header = "MY-HEADER2")] String,
        ),
    }

    let meta: MetaResponses = MyResponse::meta();
    assert_eq!(meta.responses[0].headers, &[]);
    assert_eq!(
        meta.responses[1].headers,
        vec![
            MetaHeader {
                name: "MY-HEADER1",
                description: Some("header1"),
                required: true,
                schema: MetaSchemaRef::Inline(MetaSchema {
                    format: Some("int32"),
                    ..MetaSchema::new("integer")
                })
            },
            MetaHeader {
                name: "MY-HEADER2",
                description: None,
                required: true,
                schema: MetaSchemaRef::Inline(MetaSchema::new("string"))
            }
        ]
    );

    let resp = MyResponse::A.into_response();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = MyResponse::B(88, "abc".to_string()).into_response();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("MY-HEADER1"),
        Some(&HeaderValue::from_static("88"))
    );
    assert_eq!(
        resp.headers().get("MY-HEADER2"),
        Some(&HeaderValue::from_static("abc"))
    );

    let mut resp = MyResponse::C(
        Json(BadRequestResult {
            error_code: 11,
            message: "hehe".to_string(),
        }),
        88,
        "abc".to_string(),
    )
    .into_response();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        serde_json::from_slice::<Value>(&resp.take_body().into_bytes().await.unwrap()).unwrap(),
        serde_json::json!({
            "errorCode": 11,
            "message": "hehe",
        })
    );
    assert_eq!(
        resp.headers().get("MY-HEADER1"),
        Some(&HeaderValue::from_static("88"))
    );
    assert_eq!(
        resp.headers().get("MY-HEADER2"),
        Some(&HeaderValue::from_static("abc"))
    );

    let mut resp = MyResponse::D(
        StatusCode::CONFLICT,
        PlainText("abcdef".to_string()),
        88,
        "abc".to_string(),
    )
    .into_response();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    assert_eq!(resp.take_body().into_string().await.unwrap(), "abcdef");
    assert_eq!(
        resp.headers().get("MY-HEADER1"),
        Some(&HeaderValue::from_static("88"))
    );
    assert_eq!(
        resp.headers().get("MY-HEADER2"),
        Some(&HeaderValue::from_static("abc"))
    );
}
