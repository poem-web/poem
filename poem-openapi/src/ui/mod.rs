use askama::Template;
use poem::{endpoint::make_sync, web::Html};

use crate::poem::Endpoint;

const SWAGGER_UI_JS: &str = include_str!("swagger-ui-bundle.js");
const SWAGGER_UI_CSS: &str = include_str!("swagger-ui.css");
const OAUTH2_REDIRECT_HTML: &str = include_str!("oauth2-redirect.html");

#[derive(Template)]
#[template(
    ext = "html",
    source = r#"
<html charset="UTF-8">
<head>
    <meta http-equiv="Content-Type" content="text/html;charset=utf-8">
    <title>Swagger UI</title>
    <style charset="UTF-8">{{ css|safe }}</style>
    <script charset="UTF-8">{{ script|safe }}</script>
</head>
</html>
<body>

<div id="ui"></div>
<script>
    let spec = {{ spec|safe }};

    SwaggerUIBundle({
        dom_id: '#ui',
        spec: spec,
        filter:false,
        oauth2RedirectUrl: "{{ oauth2_redirect_url|safe }}"
    })
</script>

</body>
"#
)]
struct UITemplate<'a> {
    spec: &'a str,
    script: &'static str,
    css: &'static str,
    oauth2_redirect_url: &'a str,
}

pub(crate) fn create_ui_endpoint(absolute_uri: &str, document: &str) -> impl Endpoint {
    let oauth2_redirect_url = format!("{}/oauth2-redirect.html", absolute_uri);
    let index_html = UITemplate {
        spec: document,
        script: SWAGGER_UI_JS,
        css: SWAGGER_UI_CSS,
        oauth2_redirect_url: &oauth2_redirect_url,
    }
    .render()
    .unwrap();

    poem::route()
        .at("/", make_sync(move |_| Html(index_html.clone())))
        .at(
            "/oauth2-redirect.html",
            make_sync(move |_| Html(OAUTH2_REDIRECT_HTML.to_string())),
        )
}
