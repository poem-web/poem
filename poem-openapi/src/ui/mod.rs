#[cfg(feature = "openapi-explorer")]
pub(crate) mod openapi_explorer;
#[cfg(feature = "rapidoc")]
pub(crate) mod rapidoc;
#[cfg(feature = "redoc")]
pub(crate) mod redoc;
#[cfg(feature = "scalar")]
pub(crate) mod scalar;
#[cfg(feature = "stoplight-elements")]
pub(crate) mod stoplight_elements;
#[cfg(feature = "swagger-ui")]
pub(crate) mod swagger_ui;

fn create_html_endpoint(html: impl Into<bytes::Bytes>) -> impl poem::Endpoint {
    let html = html.into();
    poem::endpoint::make_sync(move |_| {
        poem::Response::builder()
            .content_type("text/html; charset=utf-8")
            .body(html.clone())
    })
}
