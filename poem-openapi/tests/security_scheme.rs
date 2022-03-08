use poem::{
    http::header,
    test::TestClient,
    web::{cookie::Cookie, headers},
};
use poem_openapi::{
    auth::{ApiKey, Basic, Bearer},
    payload::PlainText,
    registry::{MetaOAuthFlow, MetaOAuthFlows, MetaOAuthScope, MetaSecurityScheme, Registry},
    ApiExtractor, OAuthScopes, OpenApi, OpenApiService, SecurityScheme,
};

use crate::headers::Authorization;

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
        "MySecurityScheme"
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
            .get("MySecurityScheme")
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
        registry.security_schemes.get("MySecurityScheme").unwrap(),
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

    let service = OpenApiService::new(MyApi, "test", "1.0");
    let resp = TestClient::new(service)
        .get("/test")
        .typed_header(Authorization::basic("abc", "123456"))
        .send()
        .await;
    resp.assert_status_is_ok();
    resp.assert_text("abc/123456").await;
}

#[tokio::test]
async fn bearer_auth() {
    #[derive(SecurityScheme)]
    #[oai(type = "bearer")]
    struct MySecurityScheme(Bearer);

    let mut registry = Registry::new();
    MySecurityScheme::register(&mut registry);
    assert_eq!(
        registry.security_schemes.get("MySecurityScheme").unwrap(),
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

    let service = OpenApiService::new(MyApi, "test", "1.0");
    let resp = TestClient::new(service)
        .get("/test")
        .typed_header(headers::Authorization::bearer("abcdef").unwrap())
        .send()
        .await;
    resp.assert_status_is_ok();
    resp.assert_text("abcdef").await;
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
            .get("MySecuritySchemeInHeader")
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
            .get("MySecuritySchemeInQuery")
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
            .get("MySecuritySchemeInCookie")
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

    let service = OpenApiService::new(MyApi, "test", "1.0");
    let cli = TestClient::new(service);

    let resp = cli
        .get("/header")
        .header("X-API-Key", "abcdef")
        .send()
        .await;
    resp.assert_status_is_ok();
    resp.assert_text("abcdef").await;

    let resp = cli.get("/query").query("key", &"abcdef").send().await;
    resp.assert_status_is_ok();
    resp.assert_text("abcdef").await;

    let resp = cli
        .get("/cookie")
        .header(
            header::COOKIE,
            Cookie::new_with_str("key", "abcdef").to_string(),
        )
        .send()
        .await;
    resp.assert_status_is_ok();
    resp.assert_text("abcdef").await;
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
                name: "Write",
                description: None
            }
        ]
    );
    assert_eq!(GithubScopes::Read.name(), "r_ead");
    assert_eq!(GithubScopes::Write.name(), "Write");
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
                name: "Read",
                description: Some("Read data")
            },
            MetaOAuthScope {
                name: "Write",
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
        registry.security_schemes.get("MySecurityScheme").unwrap(),
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
