use poem::{
    http::{header, Uri},
    web::cookie::Cookie,
    Endpoint, IntoEndpoint,
};
use poem_openapi::{
    auth::{ApiKey, Basic, Bearer},
    payload::PlainText,
    registry::{MetaOAuthFlow, MetaOAuthFlows, MetaOAuthScope, MetaSecurityScheme, Registry},
    ApiExtractor, OAuthScopes, OpenApi, OpenApiService, SecurityScheme,
};
use typed_headers::{http::StatusCode, Token68};

#[test]
fn rename() {
    #[derive(SecurityScheme)]
    #[oai(rename = "ABC", type = "basic")]
    struct MySecurityScheme(Basic);

    assert_eq!(MySecurityScheme::security_scheme().unwrap(), "ABC");
}

#[test]
fn default_rename() {
    #[derive(SecurityScheme)]
    #[oai(type = "basic")]
    struct MySecurityScheme(Basic);

    assert_eq!(
        MySecurityScheme::security_scheme().unwrap(),
        "my_security_scheme"
    );
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
async fn no_auth() {
    struct MyApi;

    #[OpenApi]
    impl MyApi {
        #[oai(path = "/test", method = "get")]
        async fn test(&self) -> PlainText<String> {
            PlainText("test".to_string())
        }
    }

    let service = OpenApiService::new(MyApi, "test", "1.0");
    let spec_string = service.spec();
    let spec = serde_json::from_str::<serde_json::Value>(&spec_string).unwrap();

    assert_eq!(spec["paths"]["/test"]["get"].get("security"), None);
    assert_eq!(spec["components"].get("securitySchemes"), None);
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
        async fn test(&self, auth: MySecurityScheme) -> PlainText<String> {
            PlainText(format!("{}/{}", auth.0.username, auth.0.password))
        }
    }

    let service = OpenApiService::new(MyApi, "test", "1.0").into_endpoint();
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
        .await
        .unwrap();
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
        async fn test(&self, auth: MySecurityScheme) -> PlainText<String> {
            PlainText(auth.0.token)
        }
    }

    let service = OpenApiService::new(MyApi, "test", "1.0").into_endpoint();
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
        .await
        .unwrap();
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
        async fn test_in_header(&self, auth: MySecuritySchemeInHeader) -> PlainText<String> {
            PlainText(auth.0.key)
        }

        #[oai(path = "/query", method = "get")]
        async fn test_in_query(&self, auth: MySecuritySchemeInQuery) -> PlainText<String> {
            PlainText(auth.0.key)
        }

        #[oai(path = "/cookie", method = "get")]
        async fn test_in_cookie(&self, auth: MySecuritySchemeInCookie) -> PlainText<String> {
            PlainText(auth.0.key)
        }
    }

    let service = OpenApiService::new(MyApi, "test", "1.0").into_endpoint();
    let mut resp = service
        .call(
            poem::Request::builder()
                .uri(Uri::from_static("/header"))
                .header("X-API-Key", "abcdef")
                .finish(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.take_body().into_string().await.unwrap(), "abcdef");

    let mut resp = service
        .call(
            poem::Request::builder()
                .uri(Uri::from_static("/query?key=abcdef"))
                .finish(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.take_body().into_string().await.unwrap(), "abcdef");

    let mut resp = service
        .call(
            poem::Request::builder()
                .uri(Uri::from_static("/cookie"))
                .header(
                    header::COOKIE,
                    Cookie::new_with_str("key", "abcdef").to_string(),
                )
                .finish(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.take_body().into_string().await.unwrap(), "abcdef");
}

#[tokio::test]
async fn oauth2_scopes_rename_all() {
    #[derive(OAuthScopes)]
    #[oai(rename_all = "UPPERCASE")]
    enum GithubScopes {
        Read,
        Write,
    }

    assert_eq!(
        GithubScopes::meta(),
        &[
            MetaOAuthScope {
                name: "READ",
                description: None
            },
            MetaOAuthScope {
                name: "WRITE",
                description: None
            }
        ]
    );
    assert_eq!(GithubScopes::Read.name(), "READ");
    assert_eq!(GithubScopes::Write.name(), "WRITE");
}

#[tokio::test]
async fn oauth2_scopes_rename_item() {
    #[derive(OAuthScopes)]
    enum GithubScopes {
        #[oai(rename = "r_ead")]
        Read,
        Write,
    }

    assert_eq!(
        GithubScopes::meta(),
        &[
            MetaOAuthScope {
                name: "r_ead",
                description: None
            },
            MetaOAuthScope {
                name: "write",
                description: None
            }
        ]
    );
    assert_eq!(GithubScopes::Read.name(), "r_ead");
    assert_eq!(GithubScopes::Write.name(), "write");
}

#[tokio::test]
async fn oauth2_scopes_description() {
    #[derive(OAuthScopes)]
    #[allow(dead_code)]
    enum GithubScopes {
        /// Read data
        Read,
        /// Write data
        Write,
    }

    assert_eq!(
        GithubScopes::meta(),
        &[
            MetaOAuthScope {
                name: "read",
                description: Some("Read data")
            },
            MetaOAuthScope {
                name: "write",
                description: Some("Write data")
            }
        ]
    );
}

#[tokio::test]
async fn oauth2_auth() {
    #[derive(OAuthScopes)]
    #[allow(dead_code)]
    enum GithubScopes {
        /// read data
        #[oai(rename = "read")]
        Read,

        /// write data
        #[oai(rename = "write")]
        Write,
    }

    #[derive(SecurityScheme)]
    #[oai(
        type = "oauth2",
        flows(
            implicit(
                authorization_url = "https://test.com/authorize",
                scopes = "GithubScopes"
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
                    scopes: vec![
                        MetaOAuthScope {
                            name: "read",
                            description: Some("read data")
                        },
                        MetaOAuthScope {
                            name: "write",
                            description: Some("write data")
                        },
                    ]
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
