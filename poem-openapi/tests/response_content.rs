use poem::{http::StatusCode, test::TestClient, IntoResponse};
use poem_openapi::{
    payload::{Binary, Json, Payload, PlainText},
    registry::{MetaApi, MetaMediaType, MetaResponse, MetaResponses, Registry},
    ApiResponse, Object, OpenApi, OpenApiService, ResponseContent,
};

#[derive(ResponseContent)]
enum MyResponseContent {
    A(Json<i32>),
    B(PlainText<String>),
    C(Binary<Vec<u8>>),
}

#[tokio::test]
async fn meta() {
    let media_types: Vec<MetaMediaType> = MyResponseContent::media_types();
    assert_eq!(
        media_types,
        vec![
            MetaMediaType {
                content_type: <Json<i32>>::CONTENT_TYPE,
                schema: <Json<i32>>::schema_ref()
            },
            MetaMediaType {
                content_type: <PlainText<String>>::CONTENT_TYPE,
                schema: <PlainText<String>>::schema_ref()
            },
            MetaMediaType {
                content_type: <Binary<Vec<u8>>>::CONTENT_TYPE,
                schema: <Binary<Vec<u8>>>::schema_ref()
            }
        ]
    );
}

#[tokio::test]
async fn into_response() {
    let resp = MyResponseContent::A(Json(100)).into_response();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.into_body().into_string().await.unwrap(), "100");

    let resp = MyResponseContent::B(PlainText("abc".to_string())).into_response();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.into_body().into_string().await.unwrap(), "abc");

    let resp = MyResponseContent::C(Binary(vec![1, 2, 3])).into_response();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.into_body().into_vec().await.unwrap(), vec![1, 2, 3]);
}

#[tokio::test]
async fn use_in_api_response() {
    #[derive(ApiResponse)]
    enum MyResponse {
        #[oai(status = 200)]
        Ok(MyResponseContent),
    }

    assert_eq!(
        MyResponse::meta(),
        MetaResponses {
            responses: vec![MetaResponse {
                description: "",
                status: Some(200),
                content: MyResponseContent::media_types(),
                headers: vec![]
            }]
        }
    );

    let resp = MyResponse::Ok(MyResponseContent::A(Json(100))).into_response();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.into_body().into_string().await.unwrap(), "100");
}

#[tokio::test]
async fn content_type() {
    #[derive(ResponseContent)]
    enum MyResp {
        #[oai(content_type = "application/json2")]
        A(Json<i32>),
    }

    assert_eq!(
        MyResp::media_types(),
        vec![MetaMediaType {
            content_type: "application/json2",
            schema: <Json<i32>>::schema_ref()
        }]
    );

    let resp = MyResp::A(Json(100)).into_response();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.content_type(), Some("application/json2"));
}

#[tokio::test]
async fn actual_type() {
    #[derive(Debug, Object)]
    struct MyObj {
        value: i32,
    }

    #[derive(Debug, ResponseContent)]
    enum MyRespContent {
        #[oai(actual_type = "Json<MyObj>")]
        A(Binary<Vec<u8>>),
    }

    #[derive(Debug, ApiResponse)]
    enum MyResponse {
        /// Ok
        #[oai(status = 200)]
        Ok(MyRespContent),
    }

    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self) -> MyResponse {
            MyResponse::Ok(MyRespContent::A(Binary(b"{ \"value\": 100 }".to_vec())))
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(meta.paths[0].path, "/");

    let operator = &meta.paths[0].operations[0];
    let response = &operator.responses.responses[0];

    assert_eq!(response.status, Some(200));

    let media = &response.content[0];
    assert_eq!(media.content_type, "application/json");
    assert_eq!(media.schema, <Json<MyObj>>::schema_ref());

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);
    let resp = cli.get("/").send().await;

    resp.assert_content_type("application/json");
    resp.assert_json(&serde_json::json!({ "value": 100 })).await;

    let mut registry = Registry::new();
    Api::register(&mut registry);
    let type_name: Vec<&String> = registry.schemas.keys().collect();
    assert_eq!(&type_name, &["MyObj"]);
}
