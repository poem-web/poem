use poem::test::TestClient;
#[cfg(feature = "cookie")]
use poem::{
    http::header,
    web::cookie::{Cookie, CookieJar, CookieKey},
};
#[cfg(feature = "cookie")]
use poem_openapi::param::{Cookie as ParamCookie, CookiePrivate, CookieSigned};
use poem_openapi::{
    OpenApi, OpenApiService,
    param::{Header, Path, Query},
    payload::Json,
    registry::{MetaApi, MetaParamIn, MetaSchema, MetaSchemaRef},
    types::Type,
};
use serde_json::json;

fn default_i32() -> i32 {
    999
}

#[tokio::test]
async fn param_name() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/abc", method = "get")]
        async fn test(&self, a: Query<i32>) {
            assert_eq!(a.0, 10);
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(meta.paths[0].operations[0].params[0].name, "a");

    let ep = OpenApiService::new(Api, "test", "1.0");
    TestClient::new(ep)
        .get("/abc")
        .query("a", &10)
        .send()
        .await
        .assert_status_is_ok();
}

#[tokio::test]
async fn query() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, v: Query<i32>) {
            assert_eq!(v.0, 10);
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    let param = &meta.paths[0].operations[0].params[0];
    assert_eq!(param.in_type, MetaParamIn::Query);
    assert_eq!(param.name, "v");
    assert!(param.explode);

    let api = OpenApiService::new(Api, "test", "1.0");
    TestClient::new(api)
        .get("/")
        .query("v", &10)
        .send()
        .await
        .assert_status_is_ok();
}

#[tokio::test]
async fn query_multiple_values_explode() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, v: Query<Vec<i32>>) {
            assert_eq!(v.0, vec![10, 20, 30]);
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    let param = &meta.paths[0].operations[0].params[0];
    assert_eq!(param.in_type, MetaParamIn::Query);
    assert_eq!(param.name, "v");
    assert_eq!(param.schema.unwrap_inline().ty, "array");
    assert!(param.explode);

    let api = OpenApiService::new(Api, "test", "1.0");
    TestClient::new(api)
        .get("/")
        .query("v", &10)
        .query("v", &20)
        .query("v", &30)
        .send()
        .await
        .assert_status_is_ok();
}

#[tokio::test]
async fn query_multiple_values_no_explode() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, #[oai(explode = false)] v: Query<Vec<i32>>) {
            assert_eq!(v.0, vec![10, 20, 30]);
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    let param = &meta.paths[0].operations[0].params[0];
    assert_eq!(param.in_type, MetaParamIn::Query);
    assert_eq!(param.name, "v");
    assert_eq!(param.schema.unwrap_inline().ty, "array");
    assert!(!param.explode);

    let api = OpenApiService::new(Api, "test", "1.0");
    TestClient::new(api)
        .get("/")
        .query("v", &"10,20,30")
        .send()
        .await
        .assert_status_is_ok();
}

#[tokio::test]
async fn query_default() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(
            &self,
            #[oai(default = "default_i32")] v: Query<i32>,
            #[oai(default)] k: Query<bool>,
        ) {
            assert_eq!(v.0, 999);
            assert!(!k.0);
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(
        meta.paths[0].operations[0].params[0].in_type,
        MetaParamIn::Query
    );
    assert_eq!(meta.paths[0].operations[0].params[0].name, "v");
    assert_eq!(
        meta.paths[0].operations[0].params[0].schema,
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            format: Some("int32"),
            default: Some(json!(999)),
            ..i32::schema_ref().unwrap_inline().clone()
        }))
    );

    assert_eq!(
        meta.paths[0].operations[0].params[1].in_type,
        MetaParamIn::Query
    );
    assert_eq!(meta.paths[0].operations[0].params[1].name, "k");
    assert_eq!(
        meta.paths[0].operations[0].params[1].schema,
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            default: Some(json!(false)),
            ..bool::schema_ref().unwrap_inline().clone()
        }))
    );

    let api = OpenApiService::new(Api, "test", "1.0");
    TestClient::new(api)
        .get("/")
        .send()
        .await
        .assert_status_is_ok();
}

#[tokio::test]
async fn query_ignore_case_on_param() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/abc", method = "post")]
        async fn test(&self, #[oai(ignore_case)] value: Query<i32>) -> Json<i32> {
            Json(value.0)
        }
    }

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);

    let resp = cli.post("/abc").query("value", &123).send().await;
    resp.assert_status_is_ok();
    resp.assert_json(123).await;

    let resp = cli.post("/abc").query("vaLUe", &123).send().await;
    resp.assert_status_is_ok();
    resp.assert_json(123).await;
}

#[tokio::test]
async fn query_ignore_case_on_operation() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/abc", method = "post", ignore_case)]
        async fn test(&self, value: Query<i32>) -> Json<i32> {
            Json(value.0)
        }
    }

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);

    let resp = cli.post("/abc").query("value", &123).send().await;
    resp.assert_status_is_ok();
    resp.assert_json(123).await;

    let resp = cli.post("/abc").query("vaLUe", &123).send().await;
    resp.assert_status_is_ok();
    resp.assert_json(123).await;
}

#[tokio::test]
async fn query_ignore_case_on_api() {
    struct Api;

    #[OpenApi(ignore_case)]
    impl Api {
        #[oai(path = "/abc", method = "post")]
        async fn test(&self, value: Query<i32>) -> Json<i32> {
            Json(value.0)
        }
    }

    let ep = OpenApiService::new(Api, "test", "1.0");
    let cli = TestClient::new(ep);

    let resp = cli.post("/abc").query("value", &123).send().await;
    resp.assert_status_is_ok();
    resp.assert_json(123).await;

    let resp = cli.post("/abc").query("vaLUe", &123).send().await;
    resp.assert_status_is_ok();
    resp.assert_json(123).await;
}

#[tokio::test]
async fn header() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, v: Header<i32>) {
            assert_eq!(v.0, 10);
        }
    }

    let api = OpenApiService::new(Api, "test", "1.0");
    TestClient::new(api)
        .get("/")
        .header("v", 10)
        .send()
        .await
        .assert_status_is_ok();
}

#[tokio::test]
async fn header_multiple_values() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, v: Header<Vec<i32>>) {
            assert_eq!(v.0, vec![10, 20, 30]);
        }
    }

    let api = OpenApiService::new(Api, "test", "1.0");
    TestClient::new(api)
        .get("/")
        .header("v", 10)
        .header("v", 20)
        .header("v", 30)
        .send()
        .await
        .assert_status_is_ok();
}

#[tokio::test]
async fn header_default() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, #[oai(default = "default_i32")] v: Header<i32>) {
            assert_eq!(v.0, 999);
        }
    }

    let api = OpenApiService::new(Api, "test", "1.0");
    TestClient::new(api)
        .get("/")
        .send()
        .await
        .assert_status_is_ok();
}

#[tokio::test]
async fn header_ignore_case() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, #[oai(ignore_case, name = "Value")] value: Header<i32>) {
            assert_eq!(value.0, 10);
        }
    }

    let api = OpenApiService::new(Api, "test", "1.0");
    TestClient::new(api)
        .get("/")
        .header("value", 10)
        .send()
        .await
        .assert_status_is_ok();

    let api = OpenApiService::new(Api, "test", "1.0");
    TestClient::new(api)
        .get("/")
        .header("vaLUe", 10)
        .send()
        .await
        .assert_status_is_ok();
}

#[tokio::test]
async fn path() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/k/:v", method = "get")]
        async fn test(&self, v: Path<i32>) {
            assert_eq!(v.0, 10);
        }
    }

    let api = OpenApiService::new(Api, "test", "1.0");
    TestClient::new(api)
        .get("/k/10")
        .send()
        .await
        .assert_status_is_ok();
}

#[cfg(feature = "cookie")]
#[tokio::test]
async fn cookie() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, v1: ParamCookie<i32>, v2: CookiePrivate<i32>, v3: CookieSigned<i32>) {
            assert_eq!(v1.0, 10);
            assert_eq!(v2.0, 100);
            assert_eq!(v3.0, 1000);
        }
    }

    let cookie_key = CookieKey::generate();
    let api = OpenApiService::new(Api, "test", "1.0").cookie_key(cookie_key.clone());

    let cookie_jar = CookieJar::default();
    cookie_jar.add(Cookie::new_with_str("v1", "10"));
    cookie_jar
        .private_with_key(&cookie_key)
        .add(Cookie::new_with_str("v2", "100"));
    cookie_jar
        .signed_with_key(&cookie_key)
        .add(Cookie::new_with_str("v3", "1000"));
    let cookie = format!(
        "{}; {}; {}",
        cookie_jar.get("v1").unwrap(),
        cookie_jar.get("v2").unwrap(),
        cookie_jar.get("v3").unwrap()
    );

    TestClient::new(api)
        .get("/")
        .header(header::COOKIE, cookie)
        .send()
        .await
        .assert_status_is_ok();
}

#[cfg(feature = "cookie")]
#[tokio::test]
async fn cookie_default() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, #[oai(default = "default_i32")] v: ParamCookie<i32>) {
            assert_eq!(v.0, 999);
        }
    }

    let api = OpenApiService::new(Api, "test", "1.0");
    TestClient::new(api)
        .get("/")
        .send()
        .await
        .assert_status_is_ok();
}

#[cfg(feature = "cookie")]
#[tokio::test]
async fn cookie_ignore_ascii_case() {
    struct Api;

    #[OpenApi(ignore_case)]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, v1: ParamCookie<i32>, v2: CookiePrivate<i32>, v3: CookieSigned<i32>) {
            assert_eq!(v1.0, 10);
            assert_eq!(v2.0, 100);
            assert_eq!(v3.0, 1000);
        }
    }

    let cookie_key = CookieKey::generate();
    let api = OpenApiService::new(Api, "test", "1.0").cookie_key(cookie_key.clone());
    let cli = TestClient::new(api);

    let cookie_jar = CookieJar::default();
    cookie_jar.add(Cookie::new_with_str("v1", "10"));
    cookie_jar
        .private_with_key(&cookie_key)
        .add(Cookie::new_with_str("v2", "100"));
    cookie_jar
        .signed_with_key(&cookie_key)
        .add(Cookie::new_with_str("v3", "1000"));
    let cookie = format!(
        "{}; {}; {}",
        cookie_jar.get("v1").unwrap(),
        cookie_jar.get("v2").unwrap(),
        cookie_jar.get("v3").unwrap()
    );

    cli.get("/")
        .header(header::COOKIE, cookie)
        .send()
        .await
        .assert_status_is_ok();

    let cookie_jar = CookieJar::default();
    cookie_jar.add(Cookie::new_with_str("V1", "10"));
    cookie_jar
        .private_with_key(&cookie_key)
        .add(Cookie::new_with_str("V2", "100"));
    cookie_jar
        .signed_with_key(&cookie_key)
        .add(Cookie::new_with_str("V3", "1000"));
    let cookie = format!(
        "{}; {}; {}",
        cookie_jar.get("V1").unwrap(),
        cookie_jar.get("V2").unwrap(),
        cookie_jar.get("V3").unwrap()
    );

    cli.get("/")
        .header(header::COOKIE, cookie)
        .send()
        .await
        .assert_status_is_ok();
}

#[tokio::test]
async fn deprecated() {
    struct Api;

    #[OpenApi]
    #[allow(unused_variables)]
    impl Api {
        #[oai(path = "/a", method = "get")]
        async fn a(&self, #[oai(deprecated)] v: Query<i32>) {
            todo!()
        }

        #[oai(path = "/b", method = "get")]
        async fn b(&self, v: Query<i32>) {
            todo!()
        }
    }

    let meta: MetaApi = Api::meta().remove(0);

    assert_eq!(meta.paths[0].path, "/a");
    assert!(meta.paths[0].operations[0].params[0].deprecated);

    assert_eq!(meta.paths[1].path, "/b");
    assert!(!meta.paths[1].operations[0].params[0].deprecated);
}

#[tokio::test]
async fn desc() {
    struct Api;

    #[OpenApi]
    #[allow(unused_variables)]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(
            &self,
            /// ABC
            v: Query<i32>,
        ) {
            todo!()
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(
        meta.paths[0].operations[0].params[0].in_type,
        MetaParamIn::Query
    );
    assert_eq!(meta.paths[0].operations[0].params[0].name, "v");
    assert_eq!(
        meta.paths[0].operations[0].params[0].description.as_deref(),
        Some("ABC")
    );
}

#[tokio::test]
async fn default_opt() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, #[oai(default = "default_value")] v: Query<Option<i32>>) {
            assert_eq!(v.0, Some(88));
        }
    }

    fn default_value() -> Option<i32> {
        Some(88)
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(
        meta.paths[0].operations[0].params[0]
            .schema
            .unwrap_inline()
            .default,
        Some(json!(88))
    );

    let api = OpenApiService::new(Api, "test", "1.0");
    TestClient::new(api)
        .get("/")
        .send()
        .await
        .assert_status_is_ok();
}

#[tokio::test]
async fn example() {
    struct Api;

    #[OpenApi]
    #[allow(unused_variables)]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(
            &self,
            #[oai(example = "example_value")] a: Query<Option<i32>>,
            #[oai(example)] b: Query<i32>,
        ) {
            todo!();
        }
    }

    fn example_value() -> Option<i32> {
        Some(88)
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(
        meta.paths[0].operations[0].params[0].in_type,
        MetaParamIn::Query
    );

    assert_eq!(meta.paths[0].operations[0].params[0].name, "a");
    assert_eq!(
        meta.paths[0].operations[0].params[0]
            .schema
            .unwrap_inline()
            .example,
        Some(json!(88))
    );

    assert_eq!(meta.paths[0].operations[0].params[1].name, "b");
    assert_eq!(
        meta.paths[0].operations[0].params[1]
            .schema
            .unwrap_inline()
            .example,
        Some(json!(0))
    );
}

#[tokio::test]
async fn required_params() {
    struct Api;

    #[OpenApi]
    #[allow(unused_variables)]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(
            &self,
            #[oai(default = "default_i32")] a: Query<i32>,
            b: Query<i32>,
            #[oai(default)] c: Query<bool>,
        ) {
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(
        meta.paths[0].operations[0].params[0].in_type,
        MetaParamIn::Query
    );
    assert_eq!(meta.paths[0].operations[0].params[0].name, "a");
    assert!(!meta.paths[0].operations[0].params[0].required);

    assert_eq!(
        meta.paths[0].operations[0].params[1].in_type,
        MetaParamIn::Query
    );
    assert_eq!(meta.paths[0].operations[0].params[1].name, "b");
    assert!(meta.paths[0].operations[0].params[1].required);

    assert_eq!(
        meta.paths[0].operations[0].params[2].in_type,
        MetaParamIn::Query
    );
    assert_eq!(meta.paths[0].operations[0].params[2].name, "c");
    assert!(!meta.paths[0].operations[0].params[2].required);
}

#[tokio::test]
async fn query_rename() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/abc", method = "get")]
        async fn query(&self, #[oai(name = "fooBar")] foo_bar: Query<i32>) {
            assert_eq!(foo_bar.0, 10);
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(meta.paths[0].operations[0].params[0].name, "fooBar");
    assert_eq!(
        meta.paths[0].operations[0].params[0].in_type,
        MetaParamIn::Query
    );

    let ep = OpenApiService::new(Api, "test", "1.0");
    TestClient::new(ep)
        .get("/abc")
        .query("fooBar", &10)
        .send()
        .await
        .assert_status_is_ok();
}

#[tokio::test]
async fn path_rename() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/abc/:fooBar", method = "get")]
        async fn query(&self, #[oai(name = "fooBar")] foo_bar: Path<i32>) {
            assert_eq!(foo_bar.0, 10);
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(meta.paths[0].operations[0].params[0].name, "fooBar");
    assert_eq!(
        meta.paths[0].operations[0].params[0].in_type,
        MetaParamIn::Path
    );

    let ep = OpenApiService::new(Api, "test", "1.0");
    TestClient::new(ep)
        .get("/abc/10")
        .send()
        .await
        .assert_status_is_ok();
}

#[tokio::test]
async fn header_rename() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/abc", method = "get")]
        async fn query(&self, #[oai(name = "foo-bar")] foo_bar: Header<i32>) {
            assert_eq!(foo_bar.0, 10);
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(meta.paths[0].operations[0].params[0].name, "foo-bar");
    assert_eq!(
        meta.paths[0].operations[0].params[0].in_type,
        MetaParamIn::Header
    );

    let ep = OpenApiService::new(Api, "test", "1.0");
    TestClient::new(ep)
        .get("/abc")
        .header("foo-bar", "10")
        .send()
        .await
        .assert_status_is_ok();
}

#[cfg(feature = "cookie")]
#[tokio::test]
async fn cookie_rename() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/abc", method = "get")]
        async fn query(&self, #[oai(name = "fooBar")] foo_bar: ParamCookie<i32>) {
            assert_eq!(foo_bar.0, 10);
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(meta.paths[0].operations[0].params[0].name, "fooBar");
    assert_eq!(
        meta.paths[0].operations[0].params[0].in_type,
        MetaParamIn::Cookie
    );

    let ep = OpenApiService::new(Api, "test", "1.0");
    TestClient::new(ep)
        .get("/abc")
        .header(header::COOKIE, "fooBar=10")
        .send()
        .await
        .assert_status_is_ok();
}

/// Test that hidden parameters are not included in OpenAPI schema but still
/// work at runtime
#[tokio::test]
async fn hidden_parameter() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/test", method = "get")]
        async fn test(
            &self,
            visible_param: Query<String>,
            #[oai(hidden)] hidden_param: Header<String>,
        ) -> Json<String> {
            // Both parameters should be extracted at runtime
            Json(format!(
                "visible={}, hidden={}",
                visible_param.0, hidden_param.0
            ))
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    // Only visible_param should be in the schema
    assert_eq!(meta.paths[0].operations[0].params.len(), 1);
    assert_eq!(meta.paths[0].operations[0].params[0].name, "visible_param");
    assert_eq!(
        meta.paths[0].operations[0].params[0].in_type,
        MetaParamIn::Query
    );

    // Test that both parameters still work at runtime
    let ep = OpenApiService::new(Api, "test", "1.0");
    let resp = TestClient::new(ep)
        .get("/test")
        .query("visible_param", &"hello")
        .header("hidden_param", "world")
        .send()
        .await;
    resp.assert_status_is_ok();
    resp.assert_json(&"visible=hello, hidden=world".to_string())
        .await;
}

/// Test that multiple hidden parameters work correctly
#[tokio::test]
async fn multiple_hidden_parameters() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/test/:id", method = "get")]
        async fn test(
            &self,
            id: Path<i32>,
            #[oai(hidden)] request_id: Header<String>,
            visible_query: Query<String>,
            #[oai(hidden)] internal_flag: Query<Option<bool>>,
        ) -> Json<String> {
            Json(format!(
                "id={}, request_id={}, query={}, flag={:?}",
                id.0, request_id.0, visible_query.0, internal_flag.0
            ))
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    // Only id and visible_query should be in the schema
    assert_eq!(meta.paths[0].operations[0].params.len(), 2);
    assert_eq!(meta.paths[0].operations[0].params[0].name, "id");
    assert_eq!(
        meta.paths[0].operations[0].params[0].in_type,
        MetaParamIn::Path
    );
    assert_eq!(meta.paths[0].operations[0].params[1].name, "visible_query");
    assert_eq!(
        meta.paths[0].operations[0].params[1].in_type,
        MetaParamIn::Query
    );

    // Test runtime extraction
    let ep = OpenApiService::new(Api, "test", "1.0");
    let resp = TestClient::new(ep)
        .get("/test/42")
        .query("visible_query", &"test")
        .query("internal_flag", &true)
        .header("request_id", "req-123")
        .send()
        .await;
    resp.assert_status_is_ok();
    resp.assert_json(&"id=42, request_id=req-123, query=test, flag=Some(true)".to_string())
        .await;
}

/// Test that all parameters can be hidden
#[tokio::test]
async fn all_parameters_hidden() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/test", method = "get")]
        async fn test(
            &self,
            #[oai(hidden)] hidden_query: Query<String>,
            #[oai(hidden)] hidden_header: Header<String>,
        ) -> Json<String> {
            Json(format!(
                "query={}, header={}",
                hidden_query.0, hidden_header.0
            ))
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    // No parameters should be in the schema
    assert!(meta.paths[0].operations[0].params.is_empty());

    // Test runtime extraction still works
    let ep = OpenApiService::new(Api, "test", "1.0");
    let resp = TestClient::new(ep)
        .get("/test")
        .query("hidden_query", &"q")
        .header("hidden_header", "h")
        .send()
        .await;
    resp.assert_status_is_ok();
    resp.assert_json(&"query=q, header=h".to_string()).await;
}
