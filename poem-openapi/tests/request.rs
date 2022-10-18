use poem_openapi::{
    payload::{Json, PlainText},
    registry::{MetaMediaType, MetaRequest, MetaSchema, MetaSchemaRef},
    types::ParseFromJSON,
    ApiExtractor, ApiRequest, Object,
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
        MyRequest::request_meta().unwrap(),
        MetaRequest {
            description: Some("MyRequest\n\nABC"),
            content: vec![
                MetaMediaType {
                    content_type: "application/json; charset=utf-8",
                    schema: MetaSchemaRef::Reference("CreateUser".to_string()),
                },
                MetaMediaType {
                    content_type: "text/plain; charset=utf-8",
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
        MyRequest::from_request(&request, &mut body, Default::default())
            .await
            .unwrap(),
        MyRequest::CreateByJson(Json(CreateUser {
            user: "sunli".to_string(),
            password: "123456".to_string()
        }))
    );

    let request = poem::Request::builder()
        .content_type("application/json; x=10")
        .body(
            serde_json::to_vec(&serde_json::json!({
                "user": "sunli",
                "password": "123456",
            }))
            .unwrap(),
        );
    let (request, mut body) = request.split();
    assert_eq!(
        MyRequest::from_request(&request, &mut body, Default::default())
            .await
            .unwrap(),
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
        MyRequest::from_request(&request, &mut body, Default::default())
            .await
            .unwrap(),
        MyRequest::CreateByPlainText(PlainText("abcdef".to_string()))
    );
}

#[tokio::test]
async fn generic() {
    #[derive(Debug, ApiRequest, Eq, PartialEq)]
    enum MyRequest<T: ParseFromJSON> {
        CreateByJson(Json<T>),
    }

    let request = poem::Request::builder()
        .content_type("application/json; charset=utf-8")
        .body(serde_json::to_vec(&serde_json::json!("hello")).unwrap());

    assert_eq!(
        MyRequest::<String>::request_meta().unwrap(),
        MetaRequest {
            description: None,
            content: vec![MetaMediaType {
                content_type: "application/json; charset=utf-8",
                schema: MetaSchemaRef::Inline(Box::new(MetaSchema::new("string"))),
            },],
            required: true
        }
    );

    let (request, mut body) = request.split();
    assert_eq!(
        MyRequest::<String>::from_request(&request, &mut body, Default::default())
            .await
            .unwrap(),
        MyRequest::CreateByJson(Json("hello".to_string()))
    );
}

#[tokio::test]
async fn item_content_type() {
    #[derive(Debug, ApiRequest, Eq, PartialEq)]
    enum Req {
        #[oai(content_type = "application/json+abc")]
        Create(Json<i32>),
    }

    assert_eq!(
        Req::request_meta().unwrap(),
        MetaRequest {
            description: None,
            content: vec![MetaMediaType {
                content_type: "application/json+abc",
                schema: MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format(
                    "integer", "int32"
                ))),
            },],
            required: true
        }
    );

    let request = poem::Request::builder()
        .content_type("application/json+abc")
        .body("100".to_string());
    let (request, mut body) = request.split();
    assert_eq!(
        Req::from_request(&request, &mut body, Default::default())
            .await
            .unwrap(),
        Req::Create(Json(100))
    );
}
