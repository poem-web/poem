use poem::{Error, http::StatusCode, test::TestClient};
use poem_openapi::{
    ApiResponse, Object, OpenApi, OpenApiService,
    param::Query,
    payload::{Json, Response, Xml},
};
use serde::{Deserialize, Serialize};

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
async fn xml_response_uses_serde_attributes() {
    #[derive(Object, Serialize, Deserialize)]
    struct Url {
        loc: String,
    }

    #[derive(Object, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase", rename = "urlset")]
    struct UrlSet {
        #[serde(rename = "@xmlns")]
        namespace: String,
        #[serde(rename = "url")]
        urls: Vec<Url>,
    }

    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/sitemap.xml", method = "get")]
        async fn sitemap(&self) -> Xml<UrlSet> {
            Xml(UrlSet {
                namespace: "http://www.sitemaps.org/schemas/sitemap/0.9".to_string(),
                urls: vec![
                    Url {
                        loc: "domain.com".to_string(),
                    },
                    Url {
                        loc: "domain.com/A".to_string(),
                    },
                    Url {
                        loc: "domain.com/B".to_string(),
                    },
                ],
            })
        }
    }

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);
    let resp = cli.get("/sitemap.xml").send().await;

    resp.assert_status_is_ok();
    resp.assert_content_type("application/xml; charset=utf-8");
    resp.assert_text(
        r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><url><loc>domain.com</loc></url><url><loc>domain.com/A</loc></url><url><loc>domain.com/B</loc></url></urlset>"#,
    )
    .await;
}
