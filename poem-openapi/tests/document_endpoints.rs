use poem::{Endpoint, Request, http::StatusCode};
use poem_openapi::OpenApiService;

async fn assert_document(
    endpoint: impl Endpoint,
    path: &str,
    expected: &str,
    content_type: &str,
    content_disposition: Option<&str>,
) {
    let mut first_body: Option<bytes::Bytes> = None;
    for _ in 0..3 {
        let response = endpoint
            .get_response(Request::builder().uri_str(path).finish())
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], content_type);
        assert_eq!(
            response
                .headers()
                .get("content-disposition")
                .map(|value| value.to_str().unwrap()),
            content_disposition,
        );
        let body = response.into_body().into_bytes().await.unwrap();
        assert_eq!(body.as_ref(), expected.as_bytes());
        if let Some(first_body) = &first_body {
            assert_eq!(body.as_ptr(), first_body.as_ptr());
        } else {
            first_body = Some(body);
        }
    }
}

#[tokio::test]
async fn spec_endpoints() {
    let service = OpenApiService::new((), "Document 测试", "1.0");
    assert_document(
        service.spec_endpoint(),
        "/?download=1",
        &service.spec(),
        "application/json",
        None,
    )
    .await;
    assert_document(
        service.spec_endpoint_yaml(),
        "/?download=1",
        &service.spec_yaml(),
        "application/x-yaml",
        Some("inline; filename=\"spec.yaml\""),
    )
    .await;
}

macro_rules! ui_test {
    ($feature:literal, $endpoint:ident, $html:ident) => {
        #[cfg(feature = $feature)]
        #[tokio::test]
        async fn $endpoint() {
            let service = OpenApiService::new((), "Document 测试", "1.0");
            assert_document(
                service.$endpoint(),
                "/?theme=light",
                &service.$html(),
                "text/html; charset=utf-8",
                None,
            )
            .await;
            let response = service
                .$endpoint()
                .get_response(Request::builder().uri_str("/missing").finish())
                .await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }
    };
}

ui_test!("openapi-explorer", openapi_explorer, openapi_explorer_html);
ui_test!("rapidoc", rapidoc, rapidoc_html);
ui_test!("redoc", redoc, redoc_html);
ui_test!("scalar", scalar, scalar_html);
ui_test!(
    "stoplight-elements",
    stoplight_elements,
    stoplight_elements_html
);
ui_test!("swagger-ui", swagger_ui, swagger_ui_html);

#[cfg(feature = "rapidoc")]
#[tokio::test]
async fn rapidoc_oauth_receiver() {
    let expected = include_str!("../src/ui/rapidoc/oauth-receiver.html").replace(
        "{:script}",
        include_str!("../src/ui/rapidoc/rapidoc-min.js"),
    );
    assert_document(
        OpenApiService::new((), "Document", "1.0").rapidoc(),
        "/oauth-receiver.html?code=test&state=test",
        &expected,
        "text/html; charset=utf-8",
        None,
    )
    .await;
}

#[cfg(feature = "swagger-ui")]
#[tokio::test]
async fn swagger_ui_oauth_receiver() {
    assert_document(
        OpenApiService::new((), "Document", "1.0").swagger_ui(),
        "/oauth-receiver.html?code=test&state=test",
        include_str!("../src/ui/swagger_ui/oauth-receiver.html"),
        "text/html; charset=utf-8",
        None,
    )
    .await;
}
