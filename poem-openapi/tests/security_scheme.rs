use poem::{
    http::{header, Uri},
    Endpoint, IntoEndpoint,
};
use poem_openapi::{
    auth::{Basic, Bearer},
    payload::PlainText,
    registry::{MetaSecurityScheme, Registry},
    OpenApi, OpenApiService, SecurityScheme,
};
use typed_headers::http::StatusCode;

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

#[test]
fn bearer_auth() {
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
}
