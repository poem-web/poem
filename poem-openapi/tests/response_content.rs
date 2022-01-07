use poem::{http::StatusCode, IntoResponse};
use poem_openapi::{
    payload::{Binary, Json, Payload, PlainText},
    registry::{MetaMediaType, MetaResponse, MetaResponses},
    ApiResponse, ResponseContent,
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
