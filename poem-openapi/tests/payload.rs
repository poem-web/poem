use poem::{Error, http::StatusCode, test::TestClient};
use poem_openapi::{
    ApiResponse, Object, OpenApi, OpenApiService,
    param::Query,
    payload::{Form, Json, Response},
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

#[tokio::test]
async fn form_simple() {
    #[derive(Debug, serde::Deserialize, Object)]
    struct LoginForm {
        username: String,
        password: String,
    }

    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/login", method = "post")]
        async fn login(&self, form: Form<LoginForm>) -> Json<String> {
            Json(format!("user: {}", form.username))
        }
    }

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);

    let resp = cli
        .post("/login")
        .content_type("application/x-www-form-urlencoded")
        .body("username=alice&password=secret")
        .send()
        .await;
    resp.assert_status_is_ok();
    resp.assert_json("user: alice").await;
}

#[tokio::test]
async fn form_with_array() {
    #[derive(Debug, serde::Deserialize, Object)]
    struct IdsRequest {
        ids: Vec<u32>,
    }

    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/items", method = "post")]
        async fn get_items(&self, form: Form<IdsRequest>) -> Json<Vec<u32>> {
            Json(form.ids.clone())
        }
    }

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);

    // Test with multiple values using repeated keys (correct format)
    let resp = cli
        .post("/items")
        .content_type("application/x-www-form-urlencoded")
        .body("ids=123&ids=456&ids=789")
        .send()
        .await;
    resp.assert_status_is_ok();
    resp.assert_json(&[123u32, 456, 789]).await;
}

#[tokio::test]
async fn form_array_single_value() {
    #[derive(Debug, serde::Deserialize, Object)]
    struct IdsRequest {
        ids: Vec<u32>,
    }

    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/items", method = "post")]
        async fn get_items(&self, form: Form<IdsRequest>) -> Json<Vec<u32>> {
            Json(form.ids.clone())
        }
    }

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);

    // Test with single value - serde_html_form supports this
    let resp = cli
        .post("/items")
        .content_type("application/x-www-form-urlencoded")
        .body("ids=123")
        .send()
        .await;
    resp.assert_status_is_ok();
    resp.assert_json(&[123u32]).await;
}
