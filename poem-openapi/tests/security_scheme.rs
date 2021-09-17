use poem::{
    http::{header, Uri},
    web::Cookie,
    Endpoint, IntoEndpoint,
};
use poem_openapi::{
    auth::{ApiKey, Basic, Bearer},
    payload::PlainText,
    registry::{MetaOAuthFlow, MetaOAuthFlows, MetaSecurityScheme, Registry},
    OpenApi, OpenApiService, SecurityScheme,
};
use typed_headers::{http::StatusCode, Token68};

#[test]
fn rename() {
    #[derive(SecurityScheme)]
    #[oai(name = "ABC", type = "basic")]
    struct MySecurityScheme(Basic);

    assert_eq!(MySecurityScheme::NAME, "ABC");
}

#[test]
fn default_rename() {
    #[derive(SecurityScheme)]
    #[oai(type = "basic")]
    struct MySecurityScheme(Basic);

    assert_eq!(MySecurityScheme::NAME, "my_security_scheme");
}

#[test]
fn desc() {
    /// ABC
    ///
    /// D
    #[derive(SecurityScheme)]
    #[oai(type = "basic")]
    struct MySecurityScheme(Basic);

    let mut registry = Registry::new();
    MySecurityScheme::register(&mut registry);
    assert_eq!(
        registry
            .security_schemes
            .get("my_security_scheme")
            .unwrap()
            .description,
        Some("ABC\n\nD")
    );
}

#[tokio::test]
async fn basic_auth() {
    #[derive(SecurityScheme)]
    #[oai(type = "basic")]
    struct MySecurityScheme(Basic);

    let mut registry = Registry::new();
    MySecurityScheme::register(&mut registry);
    assert_eq!(
        registry.security_schemes.get("my_security_scheme").unwrap(),
        &MetaSecurityScheme {
            ty: "http",
            description: None,
            name: None,
            key_in: None,
            scheme: Some("basic"),
            bearer_format: None,
            flows: None,
            openid_connect_url: None
        }
    );

    struct MyApi;

    #[OpenApi]
    impl MyApi {
        #[oai(path = "/test", method = "get")]
        async fn test(&self, #[oai(auth)] auth: MySecurityScheme) -> PlainText {
            format!("{}/{}", auth.0.username, auth.0.password).into()
        }
    }

    let service = OpenApiService::new(MyApi).into_endpoint();
    let mut resp = service
        .call(
            poem::Request::builder()
                .uri(Uri::from_static("/test"))
                .header(
                    header::AUTHORIZATION,
                    typed_headers::Credentials::basic("abc", "123456")
                        .unwrap()
                        .to_string(),
                )
                .finish(),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.take_body().into_string().await.unwrap(), "abc/123456");
}

#[tokio::test]
async fn bearer_auth() {
    #[derive(SecurityScheme)]
    #[oai(type = "bearer")]
    struct MySecurityScheme(Bearer);

    let mut registry = Registry::new();
    MySecurityScheme::register(&mut registry);
    assert_eq!(
        registry.security_schemes.get("my_security_scheme").unwrap(),
        &MetaSecurityScheme {
            ty: "http",
            description: None,
            name: None,
            key_in: None,
            scheme: Some("bearer"),
            bearer_format: None,
            flows: None,
            openid_connect_url: None
        }
    );

    struct MyApi;

    #[OpenApi]
    impl MyApi {
        #[oai(path = "/test", method = "get")]
        async fn test(&self, #[oai(auth)] auth: MySecurityScheme) -> PlainText {
            auth.0.token.clone().into()
        }
    }

    let service = OpenApiService::new(MyApi).into_endpoint();
    let mut resp = service
        .call(
            poem::Request::builder()
                .uri(Uri::from_static("/test"))
                .header(
                    header::AUTHORIZATION,
                    typed_headers::Credentials::bearer(Token68::new("abcdef").unwrap()).to_string(),
                )
                .finish(),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.take_body().into_string().await.unwrap(), "abcdef");
}

#[tokio::test]
async fn api_key_auth() {
    #[derive(SecurityScheme)]
    #[oai(type = "api_key", key_name = "X-API-Key", in = "header")]
    struct MySecuritySchemeInHeader(ApiKey);

    #[derive(SecurityScheme)]
    #[oai(type = "api_key", key_name = "key", in = "query")]
    struct MySecuritySchemeInQuery(ApiKey);

    #[derive(SecurityScheme)]
    #[oai(type = "api_key", key_name = "key", in = "cookie")]
    struct MySecuritySchemeInCookie(ApiKey);

    let mut registry = Registry::new();
    MySecuritySchemeInHeader::register(&mut registry);
    MySecuritySchemeInQuery::register(&mut registry);
    MySecuritySchemeInCookie::register(&mut registry);

    assert_eq!(
        registry
            .security_schemes
            .get("my_security_scheme_in_header")
            .unwrap(),
        &MetaSecurityScheme {
            ty: "apiKey",
            description: None,
            name: Some("X-API-Key"),
            key_in: Some("header"),
            scheme: None,
            bearer_format: None,
            flows: None,
            openid_connect_url: None
        }
    );

    assert_eq!(
        registry
            .security_schemes
            .get("my_security_scheme_in_query")
            .unwrap(),
        &MetaSecurityScheme {
            ty: "apiKey",
            description: None,
            name: Some("key"),
            key_in: Some("query"),
            scheme: None,
            bearer_format: None,
            flows: None,
            openid_connect_url: None
        }
    );

    assert_eq!(
        registry
            .security_schemes
            .get("my_security_scheme_in_cookie")
            .unwrap(),
        &MetaSecurityScheme {
            ty: "apiKey",
            description: None,
            name: Some("key"),
            key_in: Some("cookie"),
            scheme: None,
            bearer_format: None,
            flows: None,
            openid_connect_url: None
        }
    );

    struct MyApi;

    #[OpenApi]
    impl MyApi {
        #[oai(path = "/header", method = "get")]
        async fn test_in_header(&self, #[oai(auth)] auth: MySecuritySchemeInHeader) -> PlainText {
            auth.0.key.clone().into()
        }

        #[oai(path = "/query", method = "get")]
        async fn test_in_query(&self, #[oai(auth)] auth: MySecuritySchemeInQuery) -> PlainText {
            auth.0.key.clone().into()
        }

        #[oai(path = "/cookie", method = "get")]
        async fn test_in_cookie(&self, #[oai(auth)] auth: MySecuritySchemeInCookie) -> PlainText {
            auth.0.key.clone().into()
        }
    }

    let service = OpenApiService::new(MyApi).into_endpoint();
    let mut resp = service
        .call(
            poem::Request::builder()
                .uri(Uri::from_static("/header"))
                .header("X-API-Key", "abcdef")
                .finish(),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.take_body().into_string().await.unwrap(), "abcdef");

    let mut resp = service
        .call(
            poem::Request::builder()
                .uri(Uri::from_static("/query?key=abcdef"))
                .finish(),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.take_body().into_string().await.unwrap(), "abcdef");

    let mut resp = service
        .call(
            poem::Request::builder()
                .uri(Uri::from_static("/cookie"))
                .header(header::COOKIE, Cookie::new("key", "abcdef").to_string())
                .finish(),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.take_body().into_string().await.unwrap(), "abcdef");
}

#[tokio::test]
async fn oauth2_auth() {
    #[derive(SecurityScheme)]
    #[oai(
        type = "oauth2",
        flows(
            implicit(
                authorization_url = "https://test.com/authorize",
                scope(name = "read", desc = "read data"),
                scope(name = "write", desc = "write data")
            ),
            password(token_url = "https://test.com/token"),
            client_credentials(token_url = "https://test.com/token"),
            authorization_code(
                authorization_url = "https://test.com/authorize",
                token_url = "https://test.com/token"
            ),
        )
    )]
    struct MySecurityScheme(Bearer);

    let mut registry = Registry::new();
    MySecurityScheme::register(&mut registry);
    assert_eq!(
        registry.security_schemes.get("my_security_scheme").unwrap(),
        &MetaSecurityScheme {
            ty: "oauth2",
            description: None,
            name: None,
            key_in: None,
            scheme: None,
            bearer_format: None,
            flows: Some(MetaOAuthFlows {
                implicit: Some(MetaOAuthFlow {
                    authorization_url: Some("https://test.com/authorize"),
                    token_url: None,
                    refresh_url: None,
                    scopes: vec![("read", "read data"), ("write", "write data")]
                }),
                password: Some(MetaOAuthFlow {
                    authorization_url: None,
                    token_url: Some("https://test.com/token"),
                    refresh_url: None,
                    scopes: vec![]
                }),
                client_credentials: Some(MetaOAuthFlow {
                    authorization_url: None,
                    token_url: Some("https://test.com/token"),
                    refresh_url: None,
                    scopes: vec![]
                }),
                authorization_code: Some(MetaOAuthFlow {
                    authorization_url: Some("https://test.com/authorize"),
                    token_url: Some("https://test.com/token"),
                    refresh_url: None,
                    scopes: vec![]
                })
            }),
            openid_connect_url: None
        }
    );
}
