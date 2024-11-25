use poem::{
    http::{Method, StatusCode},
    test::TestClient,
    Error,
};
use poem_openapi::{
    param::Query,
    payload::{Json, PlainText},
    registry::MetaApi,
    types::{MaybeUndefined, ToJSON},
    ApiResponse, OpenApi, OpenApiService,
};
use serde_json::Value;

#[tokio::test]
async fn query_explode_false() {
    #[derive(ApiResponse)]
    #[oai(bad_request_handler = "bad_request_handler")]
    enum MyResponse {
        /// Ok
        #[oai(status = 200)]
        Ok(Json<Option<Value>>),
        /// Bad Request
        #[oai(status = 400)]
        BadRequest(PlainText<String>),
    }

    fn bad_request_handler(err: Error) -> MyResponse {
        MyResponse::BadRequest(PlainText(format!("!!! {err}")))
    }

    const fn none<T>() -> MaybeUndefined<T> {
        MaybeUndefined::Undefined
    }

    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/abc", method = "post")]
        async fn test(
            &self,
            #[oai(explode = false, default = "none::<Vec<u32>>")] fields: Query<
                MaybeUndefined<Vec<u32>>,
            >,
        ) -> MyResponse {
            MyResponse::Ok(Json(fields.0.to_json()))
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(meta.paths[0].path, "/abc");
    assert_eq!(meta.paths[0].operations[0].method, Method::POST);

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);

    let resp = cli.post("/abc").query("fields", &"1,2,3").send().await;
    resp.assert_status_is_ok();
    resp.assert_json(&[1, 2, 3]).await;

    let resp = cli.post("/abc").query("fields", &"").send().await;
    resp.assert_status(StatusCode::BAD_REQUEST);

    let resp = cli.post("/abc").send().await;
    resp.assert_status_is_ok();
    resp.assert_json(Value::Null).await;
}
