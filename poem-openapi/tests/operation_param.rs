use poem::{
    http::{header, Method, StatusCode, Uri},
    web::cookie::{Cookie, CookieJar, CookieKey},
    Endpoint, IntoEndpoint, Request,
};
use poem_openapi::{
    registry::{MetaApi, MetaParamIn, MetaSchema, MetaSchemaRef},
    types::Type,
    OpenApi, OpenApiService,
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
        async fn test(&self, #[oai(name = "a", in = "query")] a: i32) {
            assert_eq!(a, 10);
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(meta.paths[0].operations[0].params[0].name, "a");

    let ep = OpenApiService::new(Api).into_endpoint();
    let resp = ep
        .call(
            Request::builder()
                .method(Method::GET)
                .uri(Uri::from_static("/abc?a=10"))
                .finish(),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn query() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, #[oai(name = "v", in = "query")] v: i32) {
            assert_eq!(v, 10);
        }
    }

    let meta: MetaApi = Api::meta().remove(0);
    assert_eq!(
        meta.paths[0].operations[0].params[0].in_type,
        MetaParamIn::Query
    );
    assert_eq!(meta.paths[0].operations[0].params[0].name, "v");

    let api = OpenApiService::new(Api).into_endpoint();
    let resp = api
        .call(Request::builder().uri(Uri::from_static("/?v=10")).finish())
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn query_default() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, #[oai(name = "v", in = "query", default = "default_i32")] v: i32) {
            assert_eq!(v, 999);
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

    let api = OpenApiService::new(Api).into_endpoint();
    let resp = api.call(Request::default()).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn header() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, #[oai(name = "v", in = "header")] v: i32) {
            assert_eq!(v, 10);
        }
    }

    let api = OpenApiService::new(Api).into_endpoint();
    let resp = api.call(Request::builder().header("v", 10).finish()).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn header_default() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, #[oai(name = "v", in = "header", default = "default_i32")] v: i32) {
            assert_eq!(v, 999);
        }
    }

    let api = OpenApiService::new(Api).into_endpoint();
    let resp = api.call(Request::default()).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn path() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/k/:v", method = "get")]
        async fn test(&self, #[oai(name = "v", in = "path")] v: i32) {
            assert_eq!(v, 10);
        }
    }

    let api = OpenApiService::new(Api).into_endpoint();
    let resp = api
        .call(Request::builder().uri(Uri::from_static("/k/10")).finish())
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn cookie() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(
            &self,
            #[oai(name = "v1", in = "cookie")] v1: i32,
            #[oai(name = "v2", in = "cookie", private)] v2: i32,
            #[oai(name = "v3", in = "cookie", signed)] v3: i32,
        ) {
            assert_eq!(v1, 10);
            assert_eq!(v2, 100);
            assert_eq!(v3, 1000);
        }
    }

    let cookie_key = CookieKey::generate();
    let api = OpenApiService::new(Api)
        .cookie_key(cookie_key.clone())
        .into_endpoint();

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

    let resp = api
        .call(Request::builder().header(header::COOKIE, cookie).finish())
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn cookie_default() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, #[oai(name = "v", in = "cookie", default = "default_i32")] v: i32) {
            assert_eq!(v, 999);
        }
    }

    let api = OpenApiService::new(Api).into_endpoint();
    let resp = api.call(Request::builder().finish()).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn deprecated() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/a", method = "get")]
        async fn a(&self, #[oai(name = "v", in = "query", deprecated)] _v: i32) {
            todo!()
        }

        #[oai(path = "/b", method = "get")]
        async fn b(&self, #[oai(name = "v", in = "query")] _v: i32) {
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
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(&self, #[oai(name = "v", in = "query", desc = "ABC")] _v: i32) {
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
        meta.paths[0].operations[0].params[0].description,
        Some("ABC")
    );
}

#[tokio::test]
async fn default_opt() {
    struct Api;

    #[OpenApi]
    impl Api {
        #[oai(path = "/", method = "get")]
        async fn test(
            &self,
            #[oai(name = "v", in = "query", default = "default_value")] v: Option<i32>,
        ) {
            assert_eq!(v, Some(88));
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

    let api = OpenApiService::new(Api).into_endpoint();
    let resp = api
        .call(Request::builder().uri(Uri::from_static("/")).finish())
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
}
