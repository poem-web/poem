use poem::{Error, http::StatusCode, test::TestClient};
use poem_openapi::{
    ApiResponse, OpenApi, OpenApiService,
    param::Query,
    payload::{Json, Response},
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

    fn bad_request_handler(_: Error) -> CustomApiResponse {
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

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);

    let resp = cli.get("/a").send().await;
    resp.assert_status_is_ok();
    resp.assert_header("myheader", "abc");

    let resp = cli.get("/b").query("p1", &"qwe").send().await;
    resp.assert_status_is_ok();
    resp.assert_header("myheader", "qwe");

    let resp = cli.get("/b").send().await;
    resp.assert_status(StatusCode::BAD_REQUEST);
    resp.assert_header("MY-HEADER1", "def");
}

#[test]
fn payload_response_metadata() {
    use poem_openapi::{
        payload::{Base64, Binary, EventStream, Html, Payload, PlainText, Xml, Yaml},
        registry::{MetaMediaType, MetaResponse, MetaResponses},
    };

    #[derive(poem_openapi::Object)]
    struct TestPayload {
        value: i32,
    }

    fn check<T: ApiResponse + Payload>(content_type: &'static str) {
        assert_eq!(
            T::meta(),
            MetaResponses {
                responses: vec![MetaResponse {
                    description: "",
                    status: Some(200),
                    status_range: None,
                    content: vec![MetaMediaType {
                        content_type,
                        schema: T::schema_ref(),
                    }],
                    headers: vec![],
                }],
            }
        );
    }

    check::<Json<i32>>("application/json; charset=utf-8");
    check::<Xml<i32>>("application/xml; charset=utf-8");
    check::<Yaml<TestPayload>>("application/yaml; charset=utf-8");
    check::<PlainText<String>>("text/plain; charset=utf-8");
    check::<Html<String>>("text/html; charset=utf-8");
    check::<Binary<Vec<u8>>>("application/octet-stream");
    check::<Base64<Vec<u8>>>("text/plain; charset=utf-8");
    check::<EventStream<futures_util::stream::Empty<i32>>>("text/event-stream");
}
