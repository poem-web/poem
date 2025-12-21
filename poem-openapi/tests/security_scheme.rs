use poem::{Request, error::ResponseError, http::StatusCode, test::TestClient, web::headers};
#[cfg(feature = "cookie")]
use poem::{http::header, web::cookie::Cookie};
use poem_openapi::{
    ApiExtractor, OAuthScopes, OpenApi, OpenApiService, SecurityScheme,
    auth::{ApiKey, Basic, Bearer},
    payload::PlainText,
    registry::{MetaOAuthFlow, MetaOAuthFlows, MetaOAuthScope, MetaSecurityScheme, Registry},
};
use serde_json::{Value, json};

use crate::headers::Authorization;

#[test]
fn rename() {
    #[derive(SecurityScheme)]
    #[oai(rename = "ABC", ty = "basic")]
    #[allow(dead_code)]
    struct MySecurityScheme(Basic);

    assert_eq!(MySecurityScheme::security_schemes(), &["ABC"]);
    assert!(!MySecurityScheme::has_security_fallback());
}

#[test]
fn default_rename() {
    #[derive(SecurityScheme)]
    #[oai(ty = "basic")]
    #[allow(dead_code)]
    struct MySecurityScheme(Basic);

    assert_eq!(MySecurityScheme::security_schemes(), &["MySecurityScheme"]);
    assert!(!MySecurityScheme::has_security_fallback());
}

#[test]
fn desc() {
    /// ABC
    ///
    /// D
    #[derive(SecurityScheme)]
    #[oai(ty = "basic")]
    #[allow(dead_code)]
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
    assert!(!MySecurityScheme::has_security_fallback());
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
    #[oai(ty = "basic")]
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
    assert!(!MySecurityScheme::has_security_fallback());

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
    #[oai(ty = "bearer")]
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
    assert!(!MySecurityScheme::has_security_fallback());

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
    #[oai(ty = "api_key", key_name = "X-API-Key", key_in = "header")]
    struct MySecuritySchemeInHeader(ApiKey);

    #[derive(SecurityScheme)]
    #[oai(ty = "api_key", key_name = "key", key_in = "query")]
    struct MySecuritySchemeInQuery(ApiKey);

    #[derive(SecurityScheme)]
    #[oai(ty = "api_key", key_name = "key", key_in = "cookie")]
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

    assert!(!MySecuritySchemeInCookie::has_security_fallback());
    assert!(!MySecuritySchemeInHeader::has_security_fallback());
    assert!(!MySecuritySchemeInQuery::has_security_fallback());

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

    #[cfg(feature = "cookie")]
    {
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
        ty = "oauth2",
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
    #[allow(dead_code)]
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
    assert!(!MySecurityScheme::has_security_fallback())
}

#[tokio::test]
async fn checker_result() {
    #[derive(SecurityScheme)]
    #[oai(rename = "Checker Option", ty = "basic", checker = "extract_string")]
    struct MySecurityScheme(Basic);

    #[derive(Debug, thiserror::Error)]
    #[error("Your account is disabled")]
    struct AccountDisabledError;

    impl ResponseError for AccountDisabledError {
        fn status(&self) -> StatusCode {
            StatusCode::FORBIDDEN
        }
    }

    async fn extract_string(_req: &Request, basic: Basic) -> poem::Result<Basic> {
        if basic.username != "Disabled" {
            Ok(basic)
        } else {
            Err(AccountDisabledError)?
        }
    }

    struct MyApi;

    #[OpenApi]
    impl MyApi {
        #[oai(path = "/test", method = "get")]
        async fn test(&self, auth: MySecurityScheme) -> PlainText<String> {
            PlainText(format!("Authed: {}", auth.0.username))
        }
    }

    let service = OpenApiService::new(MyApi, "test", "1.0");
    let client = TestClient::new(service);
    let resp = client
        .get("/test")
        .typed_header(headers::Authorization::basic("Enabled", "password"))
        .send()
        .await;
    resp.assert_status_is_ok();
    resp.assert_text("Authed: Enabled").await;

    let resp = client
        .get("/test")
        .typed_header(headers::Authorization::basic("Disabled", "password"))
        .send()
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
    resp.assert_text("Your account is disabled").await;
}

#[tokio::test]
async fn checker_option() {
    #[derive(SecurityScheme)]
    #[oai(rename = "Checker Option", ty = "basic", checker = "extract_string")]
    struct MySecurityScheme(Basic);

    async fn extract_string(_req: &Request, basic: Basic) -> Option<Basic> {
        if basic.username != "Disabled" {
            Some(basic)
        } else {
            None
        }
    }

    struct MyApi;

    #[OpenApi]
    impl MyApi {
        #[oai(path = "/test", method = "get")]
        async fn test(&self, auth: MySecurityScheme) -> PlainText<String> {
            PlainText(format!("Authed: {}", auth.0.username))
        }
    }

    let service = OpenApiService::new(MyApi, "test", "1.0");
    let client = TestClient::new(service);
    let resp = client
        .get("/test")
        .typed_header(headers::Authorization::basic("Enabled", "password"))
        .send()
        .await;
    resp.assert_status_is_ok();
    resp.assert_text("Authed: Enabled").await;

    let resp = client
        .get("/test")
        .typed_header(headers::Authorization::basic("Disabled", "password"))
        .send()
        .await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn multiple_auth_methods() {
    #[derive(SecurityScheme)]
    #[oai(ty = "basic")]
    struct MySecurityScheme1(Basic);

    #[derive(SecurityScheme)]
    #[oai(ty = "api_key", key_name = "X-API-Key", key_in = "header")]
    struct MySecurityScheme2(ApiKey);

    #[derive(SecurityScheme)]
    enum MySecurityScheme {
        MySecurityScheme1(MySecurityScheme1),
        MySecurityScheme2(MySecurityScheme2),
    }

    assert_eq!(
        MySecurityScheme::security_schemes(),
        vec!["MySecurityScheme1", "MySecurityScheme2"]
    );
    assert!(!MySecurityScheme::has_security_fallback());

    struct MyApi;

    #[OpenApi]
    impl MyApi {
        #[oai(path = "/test", method = "get")]
        async fn test(&self, auth: MySecurityScheme) -> PlainText<String> {
            match auth {
                MySecurityScheme::MySecurityScheme1(auth) => {
                    PlainText(format!("basic: {}", auth.0.username))
                }
                MySecurityScheme::MySecurityScheme2(auth) => {
                    PlainText(format!("api-key: {}", auth.0.key))
                }
            }
        }
    }

    let service = OpenApiService::new(MyApi, "test", "1.0");
    let spec = serde_json::from_str::<Value>(&service.spec()).unwrap();
    let client = TestClient::new(service);
    let resp = client
        .get("/test")
        .typed_header(headers::Authorization::basic("sunli", "password"))
        .send()
        .await;
    resp.assert_status_is_ok();
    resp.assert_text("basic: sunli").await;

    let resp = client
        .get("/test")
        .header("X-API-Key", "abcdef")
        .send()
        .await;
    resp.assert_status_is_ok();
    resp.assert_text("api-key: abcdef").await;

    let resp = client.get("/test").send().await;
    resp.assert_status(StatusCode::UNAUTHORIZED);

    assert_eq!(
        &spec["paths"]["/test"]["get"]["security"],
        &json!([
          {
            "MySecurityScheme1": []
          },
          {
            "MySecurityScheme2": []
          }
        ])
    )
}

#[tokio::test]
async fn fallback() {
    #[derive(SecurityScheme)]
    #[oai(ty = "basic")]
    struct MySecuritySchemeBasic(Basic);

    #[derive(SecurityScheme)]
    enum MySecurityScheme {
        MySecuritySchemeBasic(MySecuritySchemeBasic),
        #[oai(fallback)]
        NoAuth,
    }

    let mut registry = Registry::new();
    MySecurityScheme::register(&mut registry);

    assert_eq!(
        MySecurityScheme::security_schemes(),
        vec!["MySecuritySchemeBasic"]
    );
    assert!(MySecurityScheme::has_security_fallback());

    struct MyApi;

    #[OpenApi]
    impl MyApi {
        #[oai(path = "/test", method = "get")]
        async fn test(&self, auth: MySecurityScheme) -> PlainText<String> {
            match auth {
                MySecurityScheme::MySecuritySchemeBasic(basic) => {
                    PlainText(format!("Authed: {}", basic.0.username))
                }
                MySecurityScheme::NoAuth => PlainText("NoAuth".to_string()),
            }
        }
    }

    let service = OpenApiService::new(MyApi, "test", "1.0");
    let spec = serde_json::from_str::<Value>(&service.spec()).unwrap();
    let client = TestClient::new(service);

    let resp = client
        .get("/test")
        .typed_header(headers::Authorization::basic("sunli", "password"))
        .send()
        .await;
    resp.assert_status_is_ok();
    resp.assert_text("Authed: sunli").await;

    let resp = client.get("/test").send().await;
    resp.assert_status_is_ok();
    resp.assert_text("NoAuth").await;

    assert_eq!(
        &spec["paths"]["/test"]["get"]["security"],
        &json!([
          {
            "MySecuritySchemeBasic": []
          },
          {}
        ])
    )
}

#[tokio::test]
async fn security_attribute_on_api() {
    #[derive(SecurityScheme)]
    #[oai(ty = "basic")]
    struct MySecurityScheme(Basic);

    struct MyApi;

    #[OpenApi(security = "MySecurityScheme")]
    impl MyApi {
        #[oai(path = "/protected", method = "get")]
        async fn protected(&self) -> PlainText<String> {
            PlainText("protected".to_string())
        }

        #[oai(path = "/also_protected", method = "get")]
        async fn also_protected(&self) -> PlainText<String> {
            PlainText("also protected".to_string())
        }
    }

    let service = OpenApiService::new(MyApi, "test", "1.0");
    let spec_string = service.spec();
    let spec = serde_json::from_str::<Value>(&spec_string).unwrap();

    // Both endpoints should have the security scheme applied
    assert_eq!(
        &spec["paths"]["/protected"]["get"]["security"],
        &json!([{"MySecurityScheme": []}])
    );
    assert_eq!(
        &spec["paths"]["/also_protected"]["get"]["security"],
        &json!([{"MySecurityScheme": []}])
    );

    // Security scheme should be registered
    assert!(spec["components"]["securitySchemes"]["MySecurityScheme"].is_object());
}

#[tokio::test]
async fn security_attribute_on_operation() {
    #[derive(SecurityScheme)]
    #[oai(ty = "basic")]
    struct BasicAuth(Basic);

    #[derive(SecurityScheme)]
    #[oai(ty = "api_key", key_name = "X-API-Key", key_in = "header")]
    struct ApiKeyAuth(ApiKey);

    struct MyApi;

    #[OpenApi]
    impl MyApi {
        #[oai(path = "/basic", method = "get", security = "BasicAuth")]
        async fn basic_protected(&self) -> PlainText<String> {
            PlainText("basic".to_string())
        }

        #[oai(path = "/apikey", method = "get", security = "ApiKeyAuth")]
        async fn apikey_protected(&self) -> PlainText<String> {
            PlainText("apikey".to_string())
        }

        #[oai(path = "/public", method = "get")]
        async fn public(&self) -> PlainText<String> {
            PlainText("public".to_string())
        }
    }

    let service = OpenApiService::new(MyApi, "test", "1.0");
    let spec_string = service.spec();
    let spec = serde_json::from_str::<Value>(&spec_string).unwrap();

    // Basic auth endpoint
    assert_eq!(
        &spec["paths"]["/basic"]["get"]["security"],
        &json!([{"BasicAuth": []}])
    );

    // API key auth endpoint
    assert_eq!(
        &spec["paths"]["/apikey"]["get"]["security"],
        &json!([{"ApiKeyAuth": []}])
    );

    // Public endpoint should have no security
    assert_eq!(spec["paths"]["/public"]["get"].get("security"), None);

    // Both security schemes should be registered
    assert!(spec["components"]["securitySchemes"]["BasicAuth"].is_object());
    assert!(spec["components"]["securitySchemes"]["ApiKeyAuth"].is_object());
}

#[tokio::test]
async fn security_attribute_operation_overrides_api() {
    #[derive(SecurityScheme)]
    #[oai(ty = "basic")]
    struct BasicAuth(Basic);

    #[derive(SecurityScheme)]
    #[oai(ty = "api_key", key_name = "X-API-Key", key_in = "header")]
    struct ApiKeyAuth(ApiKey);

    struct MyApi;

    // API-level security with BasicAuth
    #[OpenApi(security = "BasicAuth")]
    impl MyApi {
        // Inherits BasicAuth from API level
        #[oai(path = "/default", method = "get")]
        async fn default_auth(&self) -> PlainText<String> {
            PlainText("default".to_string())
        }

        // Overrides to ApiKeyAuth
        #[oai(path = "/override", method = "get", security = "ApiKeyAuth")]
        async fn override_auth(&self) -> PlainText<String> {
            PlainText("override".to_string())
        }
    }

    let service = OpenApiService::new(MyApi, "test", "1.0");
    let spec_string = service.spec();
    let spec = serde_json::from_str::<Value>(&spec_string).unwrap();

    // Default endpoint uses API-level BasicAuth
    assert_eq!(
        &spec["paths"]["/default"]["get"]["security"],
        &json!([{"BasicAuth": []}])
    );

    // Override endpoint uses operation-level ApiKeyAuth
    assert_eq!(
        &spec["paths"]["/override"]["get"]["security"],
        &json!([{"ApiKeyAuth": []}])
    );

    // Both security schemes should be registered
    assert!(spec["components"]["securitySchemes"]["BasicAuth"].is_object());
    assert!(spec["components"]["securitySchemes"]["ApiKeyAuth"].is_object());
}

#[tokio::test]
async fn security_attribute_with_oauth_scopes() {
    #[derive(OAuthScopes)]
    enum MyScopes {
        /// Read access
        Read,
        /// Write access  
        Write,
    }

    #[derive(SecurityScheme)]
    #[oai(
        ty = "oauth2",
        flows(implicit(
            authorization_url = "https://example.com/authorize",
            scopes = "MyScopes"
        ))
    )]
    struct OAuth2Auth(Bearer);

    struct MyApi;

    #[OpenApi(security = "OAuth2Auth", security_scope = "MyScopes::Read")]
    impl MyApi {
        // Inherits OAuth2 with Read scope from API level
        #[oai(path = "/read", method = "get")]
        async fn read_only(&self) -> PlainText<String> {
            PlainText("read".to_string())
        }

        // Overrides to require Write scope
        #[oai(
            path = "/write",
            method = "post",
            security = "OAuth2Auth",
            security_scope = "MyScopes::Write"
        )]
        async fn write_access(&self) -> PlainText<String> {
            PlainText("write".to_string())
        }
    }

    let service = OpenApiService::new(MyApi, "test", "1.0");
    let spec_string = service.spec();
    let spec = serde_json::from_str::<Value>(&spec_string).unwrap();

    // Read endpoint uses Read scope
    assert_eq!(
        &spec["paths"]["/read"]["get"]["security"],
        &json!([{"OAuth2Auth": ["Read"]}])
    );

    // Write endpoint uses Write scope
    assert_eq!(
        &spec["paths"]["/write"]["post"]["security"],
        &json!([{"OAuth2Auth": ["Write"]}])
    );
}
