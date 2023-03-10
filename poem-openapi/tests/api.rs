use poem::{
    http::{Method, StatusCode},
    test::TestClient,
    web::Data,
    Endpoint, EndpointExt, Error,
};
use poem_openapi::{
    param::{Path, Query},
    payload::{Binary, Json, Payload, PlainText},
    registry::{MetaApi, MetaExternalDocument, MetaOperation, MetaParamIn, MetaSchema, Registry},
    types::Type,
    ApiRequest, ApiResponse, Object, OpenApi, OpenApiService, Tags,
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

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);
    cli.post("/abc").send().await.assert_status_is_ok();
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
        vec!["UserOperations", "PetOperations"]
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
        vec!["CommonOperations", "UserOperations"]
    );

    let ep = OpenApiService::new(Api, "test", "1.0");
    TestClient::new(ep)
        .get("/hello/world")
        .send()
        .await
        .assert_status_is_ok();
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

    assert_eq!(
        meta_request.content[0].content_type,
        "application/json; charset=utf-8"
    );
    assert_eq!(meta_request.content[0].schema, i32::schema_ref());

    assert_eq!(
        meta_request.content[1].content_type,
        "text/plain; charset=utf-8"
    );
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

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);

    cli.get("/")
        .content_type("application/json")
        .body("100")
        .send()
        .await
        .assert_status_is_ok();

    cli.get("/")
        .content_type("application/json; x=10")
        .body("100")
        .send()
        .await
        .assert_status_is_ok();

    cli.get("/")
        .content_type("text/plain")
        .body("abc")
        .send()
        .await
        .assert_status_is_ok();

    cli.get("/")
        .content_type("application/octet-stream")
        .body(vec![1, 2, 3])
        .send()
        .await
        .assert_status_is_ok();
}

#[tokio::test]
async fn payload_request() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "post")]
        async fn test(&self, req: Json<i32>) {
            assert_eq!(req.0, 100);
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    let meta_request = meta.paths[0].operations[0].request.as_ref().unwrap();
    assert!(meta_request.required);

    assert_eq!(
        meta_request.content[0].content_type,
        "application/json; charset=utf-8"
    );
    assert_eq!(meta_request.content[0].schema, i32::schema_ref());

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);

    cli.post("/")
        .content_type("application/json")
        .body("100")
        .send()
        .await
        .assert_status_is_ok();

    cli.post("/")
        .content_type("text/plain")
        .body("100")
        .send()
        .await
        .assert_status(StatusCode::UNSUPPORTED_MEDIA_TYPE);
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
        async fn test(&self, code: Query<u16>) -> MyResponse {
            match code.0 {
                200 => MyResponse::Ok,
                409 => MyResponse::AlreadyExists(Json(code.0)),
                _ => MyResponse::Default(
                    StatusCode::from_u16(code.0).unwrap(),
                    PlainText(format!("code: {}", code.0)),
                ),
            }
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    let meta_responses = &meta.paths[0].operations[0].responses;
    assert_eq!(meta_responses.responses[0].description, "Ok");
    assert_eq!(meta_responses.responses[0].status, Some(200));
    assert!(meta_responses.responses[0].content.is_empty());

    assert_eq!(meta_responses.responses[1].description, "Already exists");
    assert_eq!(meta_responses.responses[1].status, Some(409));
    assert_eq!(
        meta_responses.responses[1].content[0].content_type,
        "application/json; charset=utf-8"
    );
    assert_eq!(
        meta_responses.responses[1].content[0].schema,
        u16::schema_ref()
    );

    assert_eq!(meta_responses.responses[2].description, "Default");
    assert_eq!(meta_responses.responses[2].status, None);
    assert_eq!(
        meta_responses.responses[2].content[0].content_type,
        "text/plain; charset=utf-8"
    );
    assert_eq!(
        meta_responses.responses[2].content[0].schema,
        String::schema_ref()
    );

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);

    let resp = cli.get("/").query("code", &200).send().await;
    resp.assert_status_is_ok();
    resp.assert_text("").await;

    let resp = cli.get("/").query("code", &409).send().await;
    resp.assert_status(StatusCode::CONFLICT);
    resp.assert_content_type("application/json; charset=utf-8");
    resp.assert_text("409").await;

    let resp = cli.get("/").query("code", &404).send().await;
    resp.assert_status(StatusCode::NOT_FOUND);
    resp.assert_content_type("text/plain; charset=utf-8");
    resp.assert_text("code: 404").await;
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

    fn bad_request_handler(err: Error) -> MyResponse {
        MyResponse::BadRequest(PlainText(format!("!!! {err}")))
    }

    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, code: Query<u16>) -> MyResponse {
            MyResponse::Ok(PlainText(format!("code: {}", code.0)))
        }
    }

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);

    let resp = cli.get("/").query("code", &200).send().await;
    resp.assert_status_is_ok();
    resp.assert_content_type("text/plain; charset=utf-8");
    resp.assert_text("code: 200").await;

    let resp = cli.get("/").send().await;
    resp.assert_status(StatusCode::BAD_REQUEST);
    resp.assert_content_type("text/plain; charset=utf-8");
    resp.assert_text(
        r#"!!! failed to parse parameter `code`: Type "integer(uint16)" expects an input value."#,
    )
    .await;
}

#[tokio::test]
async fn bad_request_handler_for_validator() {
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

    fn bad_request_handler(err: Error) -> MyResponse {
        MyResponse::BadRequest(PlainText(format!("!!! {err}")))
    }

    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(
            &self,
            #[oai(validator(maximum(value = "100")))] code: Query<u16>,
        ) -> MyResponse {
            MyResponse::Ok(PlainText(format!("code: {}", code.0)))
        }
    }

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);

    let resp = cli.get("/").query("code", &50).send().await;
    resp.assert_status_is_ok();
    resp.assert_content_type("text/plain; charset=utf-8");
    resp.assert_text("code: 50").await;

    let resp = cli.get("/").query("code", &200).send().await;
    resp.assert_status(StatusCode::BAD_REQUEST);
    resp.assert_content_type("text/plain; charset=utf-8");
    resp.assert_text(r#"!!! failed to parse parameter `code`: verification failed. maximum(100, exclusive: false)"#).await;
}

#[tokio::test]
async fn poem_extract() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/test1", method = "get")]
        async fn test1(&self, data: Data<&i32>) {
            assert_eq!(*data.0, 100);
        }

        #[oai(path = "/test2", method = "get")]
        async fn test2(&self, Data(value): Data<&i32>) {
            assert_eq!(*value, 100);
        }

        #[oai(path = "/test3/:user_id", method = "get")]
        async fn test3(&self, Path(user_id): Path<i32>) {
            assert_eq!(user_id, 7);
        }
    }

    let ep = OpenApiService::new(Api, "test", "1.0").data(100i32);
    let client = TestClient::new(ep);
    client.get("/test1").send().await.assert_status_is_ok();
    client.get("/test2").send().await.assert_status_is_ok();
    client.get("/test3/7").send().await.assert_status_is_ok();
}

#[tokio::test]
async fn returning_borrowed_value() {
    struct Api {
        value1: i32,
        value2: String,
        values: Vec<i32>,
    }

    #[OpenApi]
    impl Api {
        #[oai(path = "/value1", method = "get")]
        async fn value1(&self) -> Json<&i32> {
            Json(&self.value1)
        }

        #[oai(path = "/value2", method = "get")]
        async fn value2(&self) -> Json<&String> {
            Json(&self.value2)
        }

        #[oai(path = "/value3", method = "get")]
        async fn value3<'a>(&self, data: Data<&'a i32>) -> Json<&'a i32> {
            Json(&data)
        }

        #[oai(path = "/value4", method = "get")]
        async fn value4<'a>(&self, Data(value): Data<&'a i32>) -> Json<&'a i32> {
            Json(value)
        }

        #[oai(path = "/values", method = "get")]
        async fn values(&self) -> Json<&[i32]> {
            Json(&self.values)
        }
    }

    let ep = OpenApiService::new(
        Api {
            value1: 999,
            value2: "abc".to_string(),
            values: vec![1, 2, 3, 4, 5],
        },
        "test",
        "1.0",
    )
    .data(888i32);
    let cli = TestClient::new(ep);

    let resp = cli.get("/value1").send().await;
    resp.assert_status_is_ok();
    resp.assert_text("999").await;

    let resp = cli.get("/value2").send().await;
    resp.assert_status_is_ok();
    resp.assert_text("\"abc\"").await;

    let resp = cli.get("/value3").send().await;
    resp.assert_status_is_ok();
    resp.assert_text("888").await;

    let resp = cli.get("/value4").send().await;
    resp.assert_status_is_ok();
    resp.assert_text("888").await;

    let resp = cli.get("/values").send().await;
    resp.assert_status_is_ok();
    resp.assert_text("[1,2,3,4,5]").await;
}

#[tokio::test]
async fn external_docs() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(
            path = "/",
            method = "get",
            external_docs = "https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md"
        )]
        async fn test(&self) {}
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(
        meta.paths[0].operations[0].external_docs,
        Some(MetaExternalDocument {
            url: "https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md"
                .to_string(),
            description: None
        })
    );
}

#[tokio::test]
async fn generic() {
    trait MyApiPort: Send + Sync + 'static {
        fn test(&self) -> String;
    }

    struct MyApiA;

    impl MyApiPort for MyApiA {
        fn test(&self) -> String {
            "test".to_string()
        }
    }

    struct MyOpenApi<MyApi> {
        api: MyApi,
    }

    #[OpenApi]
    impl<MyApi: MyApiPort> MyOpenApi<MyApi> {
        #[oai(path = "/some_call", method = "get")]
        async fn some_call(&self) -> Json<String> {
            Json(self.api.test())
        }
    }

    let ep = OpenApiService::new(MyOpenApi { api: MyApiA }, "test", "1.0");
    let cli = TestClient::new(ep);

    let resp = cli.get("/some_call").send().await;
    resp.assert_status_is_ok();
    resp.assert_json("test").await;
}

#[tokio::test]
async fn extra_response_headers_on_operation() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(
            path = "/",
            method = "get",
            response_header(name = "A1", type = "String", description = "abc"),
            response_header(name = "a2", type = "i32", deprecated = true)
        )]
        async fn test(&self) {}
    }

    let meta: MetaApi = Api::meta().remove(0);

    let header = &meta.paths[0].operations[0].responses.responses[0].headers[0];
    assert_eq!(header.name, "A1");
    assert_eq!(header.description.as_deref(), Some("abc"));
    assert!(!header.deprecated);
    assert_eq!(header.schema, String::schema_ref());

    let header = &meta.paths[0].operations[0].responses.responses[0].headers[1];
    assert_eq!(header.name, "A2");
    assert_eq!(header.description, None);
    assert!(header.deprecated);
    assert_eq!(header.schema, i32::schema_ref());
}

#[tokio::test]
async fn extra_response_headers_on_api() {
    struct Api;

    #[OpenApi(
        response_header(name = "A1", type = "String", description = "abc"),
        response_header(name = "a2", type = "i32", deprecated = true)
    )]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self) {}
    }

    let meta: MetaApi = Api::meta().remove(0);

    let header = &meta.paths[0].operations[0].responses.responses[0].headers[0];
    assert_eq!(header.name, "A1");
    assert_eq!(header.description.as_deref(), Some("abc"));
    assert!(!header.deprecated);
    assert_eq!(header.schema, String::schema_ref());

    let header = &meta.paths[0].operations[0].responses.responses[0].headers[1];
    assert_eq!(header.name, "A2");
    assert_eq!(header.description, None);
    assert!(header.deprecated);
    assert_eq!(header.schema, i32::schema_ref());
}

#[tokio::test]
async fn extra_request_headers_on_operation() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(
            path = "/",
            method = "get",
            request_header(name = "A1", type = "String", description = "abc"),
            request_header(name = "a2", type = "i32", deprecated = true)
        )]
        async fn test(&self) {}
    }

    let meta: MetaApi = Api::meta().remove(0);

    let params = &meta.paths[0].operations[0].params[0];
    assert_eq!(params.name, "A1");
    assert_eq!(params.schema, String::schema_ref());
    assert_eq!(params.in_type, MetaParamIn::Header);
    assert_eq!(params.description.as_deref(), Some("abc"));
    assert!(params.required);
    assert!(!params.deprecated);

    let params = &meta.paths[0].operations[0].params[1];
    assert_eq!(params.name, "A2");
    assert_eq!(params.schema, i32::schema_ref());
    assert_eq!(params.in_type, MetaParamIn::Header);
    assert_eq!(params.description, None);
    assert!(params.required);
    assert!(params.deprecated);
}

#[tokio::test]
async fn extra_request_headers_on_api() {
    struct Api;

    #[OpenApi(
        request_header(name = "A1", type = "String", description = "abc"),
        request_header(name = "a2", type = "i32", deprecated = true)
    )]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self) {}
    }

    let meta: MetaApi = Api::meta().remove(0);

    let params = &meta.paths[0].operations[0].params[0];
    assert_eq!(params.name, "A1");
    assert_eq!(params.schema, String::schema_ref());
    assert_eq!(params.in_type, MetaParamIn::Header);
    assert_eq!(params.description.as_deref(), Some("abc"));
    assert!(params.required);
    assert!(!params.deprecated);

    let params = &meta.paths[0].operations[0].params[1];
    assert_eq!(params.name, "A2");
    assert_eq!(params.schema, i32::schema_ref());
    assert_eq!(params.in_type, MetaParamIn::Header);
    assert_eq!(params.description, None);
    assert!(params.required);
    assert!(params.deprecated);
}

#[tokio::test]
async fn multiple_methods() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/abc", method = "post", method = "put")]
        async fn test(&self) {}
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(meta.paths[0].path, "/abc");
    assert_eq!(meta.paths[0].operations[0].method, Method::POST);
    assert_eq!(meta.paths[0].operations[1].method, Method::PUT);

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);

    cli.post("/abc").send().await.assert_status_is_ok();
    cli.put("/abc").send().await.assert_status_is_ok();
}

#[tokio::test]
async fn actual_type() {
    #[derive(Debug, Object)]
    struct MyObj {
        value: i32,
    }

    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get", actual_type = "Json<MyObj>")]
        async fn test(&self) -> Binary<Vec<u8>> {
            Binary(b"{ \"value\": 100 }".to_vec())
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(meta.paths[0].path, "/");

    let operator = &meta.paths[0].operations[0];
    let response = &operator.responses.responses[0];

    assert_eq!(response.status, Some(200));

    let media = &response.content[0];
    assert_eq!(media.content_type, "application/json; charset=utf-8");
    assert_eq!(media.schema, <Json<MyObj>>::schema_ref());

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);
    let resp = cli.get("/").send().await;

    resp.assert_content_type("application/json; charset=utf-8");
    resp.assert_json(&serde_json::json!({ "value": 100 })).await;

    let mut registry = Registry::new();
    Api::register(&mut registry);
    let type_name: Vec<&String> = registry.schemas.keys().collect();
    assert_eq!(&type_name, &["MyObj"]);
}

#[tokio::test]
async fn code_samples() {
    const JS_SOURCE: &str = "
    J
    S

    JavaScript
    ";

    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(
            path = "/test1",
            method = "get",
            code_sample(lang = "js", source = "JS_SOURCE")
        )]
        async fn test1(&self) {}

        #[oai(
            path = "/test2",
            method = "get",
            code_sample(lang = "rust", label = "Rust lang", source = "\"Rust Source\""),
            code_sample(lang = "go", label = "Go", source = "\"Google Go\"")
        )]
        async fn test2(&self) {}
    }

    let meta: MetaApi = Api::meta().remove(0);
    let path = &meta.paths[0];
    let operator: &MetaOperation = &path.operations[0];
    assert_eq!(path.path, "/test1");
    let code_sample = &operator.code_samples[0];
    assert_eq!(code_sample.lang, "js");
    assert_eq!(code_sample.label, None);
    assert_eq!(code_sample.source, JS_SOURCE);

    let path = &meta.paths[1];
    let operator: &MetaOperation = &path.operations[0];
    assert_eq!(path.path, "/test2");
    let code_sample = &operator.code_samples[0];
    assert_eq!(code_sample.lang, "rust");
    assert_eq!(code_sample.label, Some("Rust lang"));
    assert_eq!(code_sample.source, "Rust Source");
    let code_sample = &operator.code_samples[1];
    assert_eq!(code_sample.lang, "go");
    assert_eq!(code_sample.label, Some("Go"));
    assert_eq!(code_sample.source, "Google Go");
}

#[tokio::test]
async fn hidden() {
    #[derive(Debug, Object)]
    struct MyObj1 {
        value: i32,
    }

    #[derive(Debug, Object)]
    struct MyObj2 {
        value: i32,
    }

    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/api1", method = "get", hidden)]
        async fn api1(&self, req: Json<MyObj1>) -> Json<i32> {
            Json(req.0.value)
        }

        #[oai(path = "/api2", method = "get")]
        async fn api2(&self, req: Json<MyObj2>) -> Json<i32> {
            Json(req.0.value)
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(meta.paths.len(), 1);
    let path = &meta.paths[0];
    assert_eq!(path.path, "/api2");

    let mut registry = Registry::new();
    Api::register(&mut registry);

    assert!(!registry.schemas.contains_key("MyObj1"));
    assert!(registry.schemas.contains_key("MyObj2"));
}

#[test]
fn issue_405() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(
            path = "/hello",
            method = "get",
            operation_id = "hello",
            transform = "my_transformer"
        )]
        async fn index(&self) -> PlainText<String> {
            PlainText("hello, world!".to_string())
        }
    }

    fn my_transformer(ep: impl Endpoint) -> impl Endpoint {
        ep.map_to_response()
    }
}

#[tokio::test]
async fn issue_489() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/hello", method = "get")]
        async fn get_hello(&self) {}

        #[oai(path = "/hello", method = "delete")]
        async fn delete_hello(&self) {}

        #[oai(path = "/goodbye", method = "get")]
        async fn get_goodbye(&self) {}
    }

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);
    cli.delete("/goodbye")
        .send()
        .await
        .assert_status(StatusCode::METHOD_NOT_ALLOWED);
}
