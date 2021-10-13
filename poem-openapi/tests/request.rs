use poem_openapi::{
    payload::{Json, PlainText},
    registry::{MetaMediaType, MetaRequest, MetaSchema, MetaSchemaRef},
    ApiRequest, Object,
};

#[derive(Debug, Object, Eq, PartialEq)]
struct CreateUser {
    user: String,
    password: String,
}

/// MyRequest
///
/// ABC
#[derive(Debug, ApiRequest, Eq, PartialEq)]
enum MyRequest {
    CreateByJson(Json<CreateUser>),
    CreateByPlainText(PlainText<String>),
}

#[test]
fn meta() {
    assert_eq!(
        MyRequest::meta(),
        MetaRequest {
            description: Some("MyRequest\n\nABC"),
            content: vec![
                MetaMediaType {
                    content_type: "application/json",
                    schema: MetaSchemaRef::Reference("CreateUser"),
                },
                MetaMediaType {
                    content_type: "text/plain",
                    schema: MetaSchemaRef::Inline(Box::new(MetaSchema::new("string"))),
                }
            ],
            required: true
        }
    );
}

#[tokio::test]
async fn from_request() {
    let request = poem::Request::builder()
        .content_type("application/json")
        .body(
            serde_json::to_vec(&serde_json::json!({
                "user": "sunli",
                "password": "123456",
            }))
            .unwrap(),
        );
    let (request, mut body) = request.split();
    assert_eq!(
        MyRequest::from_request(&request, &mut body).await.unwrap(),
        MyRequest::CreateByJson(Json(CreateUser {
            user: "sunli".to_string(),
            password: "123456".to_string()
        }))
    );

    let request = poem::Request::builder()
        .content_type("text/plain")
        .body("abcdef".to_string());
    let (request, mut body) = request.split();
    assert_eq!(
        MyRequest::from_request(&request, &mut body).await.unwrap(),
        MyRequest::CreateByPlainText(PlainText("abcdef".to_string()))
    );
}
