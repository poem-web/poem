use poem::{
    http::{StatusCode, Uri},
    Endpoint, IntoEndpoint, Request,
};
use poem_openapi::{
    param::Query,
    payload::{Json, Response},
    ApiResponse, OpenApi, OpenApiService, ParseRequestError,
};

#[tokio::test]
async fn response_wrapper() {
    #[derive(ApiResponse, Debug, Eq, PartialEq)]
    #[oai(bad_request_handler = "bad_request_handler")]
    #[allow(dead_code)]
    pub enum CustomApiResponse {
        #[oai(status = 200)]
        Ok,
        #[oai(status = 400)]
        BadRequest(#[oai(header = "MY-HEADER1")] String),
    }

    fn bad_request_handler(_: ParseRequestError) -> CustomApiResponse {
        CustomApiResponse::BadRequest("def".to_string())
    }

    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/a", method = "get")]
        async fn a(&self) -> Response<Json<i32>> {
            Response::new(Json(100)).header("myheader", "abc")
        }

        #[oai(path = "/b", method = "get")]
        async fn b(&self, p1: Query<String>) -> Response<CustomApiResponse> {
            Response::new(CustomApiResponse::Ok).header("myheader", p1.0)
        }
    }

    let ep = OpenApiService::new(Api, "test", "1.0").into_endpoint();

    let resp = ep
        .call(Request::builder().uri(Uri::from_static("/a")).finish())
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.header("myheader"), Some("abc"));

    let resp = ep
        .call(
            Request::builder()
                .uri(Uri::from_static("/b?p1=qwe"))
                .finish(),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.header("myheader"), Some("qwe"));

    let resp = ep
        .call(Request::builder().uri(Uri::from_static("/b")).finish())
        .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(resp.header("MY-HEADER1"), Some("def"));
}
