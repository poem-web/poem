use poem::{
    http::{Method, StatusCode, Uri},
    web::Data,
    Endpoint, EndpointExt, IntoEndpoint,
};
use poem_openapi::{
    payload::{Binary, Json, PlainText},
    registry::{MetaApi, MetaSchema},
    types::Type,
    ApiRequest, ApiResponse, OpenApi, OpenApiService, ParseRequestError, Tags,
};

#[tokio::test]
async fn path_and_method() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/abc", method = "post")]
        async fn test(&self) {}
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(meta.paths[0].path, "/abc");
    assert_eq!(meta.paths[0].operations[0].method, Method::POST);

    let ep = OpenApiService::new(Api, "test", "1.0").into_endpoint();
    let resp = ep
        .call(
            poem::Request::builder()
                .method(Method::POST)
                .uri(Uri::from_static("/abc"))
                .finish(),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[test]
fn deprecated() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/abc", method = "get", deprecated)]
        async fn test(&self) {}
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert!(meta.paths[0].operations[0].deprecated);
}

#[test]
fn tag() {
    #[derive(Tags)]
    enum MyTags {
        /// User operations
        UserOperations,
        /// Pet operations
        PetOperations,
    }

    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(
            path = "/abc",
            method = "get",
            tag = "MyTags::UserOperations",
            tag = "MyTags::PetOperations"
        )]
        async fn test(&self) {}
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(
        meta.paths[0].operations[0].tags,
        vec!["user_operations", "pet_operations"]
    );
}

#[tokio::test]
async fn common_attributes() {
    #[derive(Tags)]
    enum MyTags {
        UserOperations,
        CommonOperations,
    }

    struct Api;

    #[OpenApi(prefix_path = "/hello", tag = "MyTags::CommonOperations")]
    impl Api {
        #[oai(path = "/world", method = "get", tag = "MyTags::UserOperations")]
        async fn test(&self) {}
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(meta.paths[0].path, "/hello/world");
    assert_eq!(
        meta.paths[0].operations[0].tags,
        vec!["common_operations", "user_operations"]
    );

    let ep = OpenApiService::new(Api, "test", "1.0").into_endpoint();
    let resp = ep
        .call(
            poem::Request::builder()
                .method(Method::GET)
                .uri(Uri::from_static("/hello/world"))
                .finish(),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn request() {
    /// Test request
    #[derive(ApiRequest)]
    enum MyRequest {
        Json(Json<i32>),
        PlainText(PlainText<String>),
        Binary(Binary<Vec<u8>>),
    }

    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, req: MyRequest) {
            match req {
                MyRequest::Json(value) => {
                    assert_eq!(value.0, 100);
                }
                MyRequest::PlainText(value) => {
                    assert_eq!(value.0, "abc");
                }
                MyRequest::Binary(value) => {
                    assert_eq!(value.0, vec![1, 2, 3]);
                }
            }
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    let meta_request = meta.paths[0].operations[0].request.as_ref().unwrap();
    assert!(meta_request.required);
    assert_eq!(meta_request.description, Some("Test request"));

    assert_eq!(meta_request.content[0].content_type, "application/json");
    assert_eq!(meta_request.content[0].schema, i32::schema_ref());

    assert_eq!(meta_request.content[1].content_type, "text/plain");
    assert_eq!(meta_request.content[1].schema, String::schema_ref());

    assert_eq!(
        meta_request.content[2].content_type,
        "application/octet-stream"
    );
    assert_eq!(
        meta_request.content[2].schema.unwrap_inline(),
        &MetaSchema {
            format: Some("binary"),
            ..MetaSchema::new("string")
        }
    );

    let ep = OpenApiService::new(Api, "test", "1.0").into_endpoint();
    let resp = ep
        .call(
            poem::Request::builder()
                .method(Method::GET)
                .uri(Uri::from_static("/"))
                .content_type("application/json")
                .body("100"),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = ep
        .call(
            poem::Request::builder()
                .method(Method::GET)
                .uri(Uri::from_static("/"))
                .content_type("text/plain")
                .body("abc"),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = ep
        .call(
            poem::Request::builder()
                .method(Method::GET)
                .uri(Uri::from_static("/"))
                .content_type("application/octet-stream")
                .body(vec![1, 2, 3]),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn payload_request() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, req: Json<i32>) {
            assert_eq!(req.0, 100);
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    let meta_request = meta.paths[0].operations[0].request.as_ref().unwrap();
    assert!(meta_request.required);

    assert_eq!(meta_request.content[0].content_type, "application/json");
    assert_eq!(meta_request.content[0].schema, i32::schema_ref());

    let ep = OpenApiService::new(Api, "test", "1.0").into_endpoint();
    let resp = ep
        .call(
            poem::Request::builder()
                .method(Method::GET)
                .uri(Uri::from_static("/"))
                .content_type("application/json")
                .body("100"),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = ep
        .call(
            poem::Request::builder()
                .method(Method::GET)
                .uri(Uri::from_static("/"))
                .content_type("text/plain")
                .body("100"),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn response() {
    #[derive(ApiResponse)]
    enum MyResponse {
        /// Ok
        #[oai(status = 200)]
        Ok,
        /// Already exists
        #[oai(status = 409)]
        AlreadyExists(Json<u16>),
        /// Default
        Default(StatusCode, PlainText<String>),
    }

    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, #[oai(name = "code", in = "query")] code: u16) -> MyResponse {
            match code {
                200 => MyResponse::Ok,
                409 => MyResponse::AlreadyExists(Json(code)),
                _ => MyResponse::Default(
                    StatusCode::from_u16(code).unwrap(),
                    PlainText(format!("code: {}", code)),
                ),
            }
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    let meta_responses = &meta.paths[0].operations[0].responses;
    assert_eq!(meta_responses.responses[0].description, Some("Ok"));
    assert_eq!(meta_responses.responses[0].status, Some(200));
    assert!(meta_responses.responses[0].content.is_empty());

    assert_eq!(
        meta_responses.responses[1].description,
        Some("Already exists")
    );
    assert_eq!(meta_responses.responses[1].status, Some(409));
    assert_eq!(
        meta_responses.responses[1].content[0].content_type,
        "application/json"
    );
    assert_eq!(
        meta_responses.responses[1].content[0].schema,
        u16::schema_ref()
    );

    assert_eq!(meta_responses.responses[2].description, Some("Default"));
    assert_eq!(meta_responses.responses[2].status, None);
    assert_eq!(
        meta_responses.responses[2].content[0].content_type,
        "text/plain"
    );
    assert_eq!(
        meta_responses.responses[2].content[0].schema,
        String::schema_ref()
    );

    let ep = OpenApiService::new(Api, "test", "1.0").into_endpoint();

    let mut resp = ep
        .call(
            poem::Request::builder()
                .method(Method::GET)
                .uri(Uri::from_static("/?code=200"))
                .finish(),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.take_body().into_string().await.unwrap(), "");

    let mut resp = ep
        .call(
            poem::Request::builder()
                .method(Method::GET)
                .uri(Uri::from_static("/?code=409"))
                .finish(),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    assert_eq!(resp.content_type(), Some("application/json"));
    assert_eq!(resp.take_body().into_string().await.unwrap(), "409");

    let mut resp = ep
        .call(
            poem::Request::builder()
                .method(Method::GET)
                .uri(Uri::from_static("/?code=404"))
                .finish(),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert_eq!(resp.content_type(), Some("text/plain"));
    assert_eq!(resp.take_body().into_string().await.unwrap(), "code: 404");
}

#[tokio::test]
async fn bad_request_handler() {
    #[derive(ApiResponse)]
    #[oai(bad_request_handler = "bad_request_handler")]
    enum MyResponse {
        /// Ok
        #[oai(status = 200)]
        Ok(PlainText<String>),
        /// Already exists
        #[oai(status = 400)]
        BadRequest(PlainText<String>),
    }

    fn bad_request_handler(err: ParseRequestError) -> MyResponse {
        MyResponse::BadRequest(PlainText(format!("!!! {}", err.to_string())))
    }

    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, #[oai(name = "code", in = "query")] code: u16) -> MyResponse {
            MyResponse::Ok(PlainText(format!("code: {}", code)))
        }
    }

    let ep = OpenApiService::new(Api, "test", "1.0").into_endpoint();

    let mut resp = ep
        .call(
            poem::Request::builder()
                .method(Method::GET)
                .uri(Uri::from_static("/?code=200"))
                .finish(),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.content_type(), Some("text/plain"));
    assert_eq!(resp.take_body().into_string().await.unwrap(), "code: 200");

    let mut resp = ep
        .call(
            poem::Request::builder()
                .method(Method::GET)
                .uri(Uri::from_static("/"))
                .finish(),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(resp.content_type(), Some("text/plain"));
    assert_eq!(
        resp.take_body().into_string().await.unwrap(),
        r#"!!! failed to parse param `code`: Type "integer(uint16)" expects an input value."#
    );
}

#[tokio::test]
async fn poem_extract() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, #[oai(extract)] data: Data<&i32>) {
            assert_eq!(*data.0, 100);
        }
    }

    let ep = OpenApiService::new(Api, "test", "1.0")
        .data(100i32)
        .into_endpoint();
    let resp = ep
        .call(
            poem::Request::builder()
                .method(Method::GET)
                .uri(Uri::from_static("/"))
                .finish(),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
}
