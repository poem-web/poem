use poem::Endpoint;

use super::create_html_endpoint;

const SWAGGER_UI_JS: &str = include_str!("swagger-ui-bundle.js");
const SWAGGER_UI_CSS: &str = include_str!("swagger-ui.css");
const OAUTH_RECEIVER_HTML: &str = include_str!("oauth-receiver.html");

const SWAGGER_UI_TEMPLATE: &str = r#"
<html charset="UTF-8">
<head>
    <meta http-equiv="Content-Type" content="text/html;charset=utf-8">
    <title>Swagger UI</title>
    <style charset="UTF-8">{:style}</style>
    <script charset="UTF-8">{:script}</script>
</head>
<body>

<div id="ui"></div>
<script>
    let spec = {:spec};
    let oauth2RedirectUrl;

    let query = window.location.href.indexOf("?");
    if (query > 0) {
        oauth2RedirectUrl = window.location.href.substring(0, query);
    } else {
        oauth2RedirectUrl = window.location.href;
    }

    if (!oauth2RedirectUrl.endsWith("/")) {
        oauth2RedirectUrl += "/";
    }
    oauth2RedirectUrl += "oauth-receiver.html";

    SwaggerUIBundle({
        dom_id: '#ui',
        spec: spec,
        filter: false,
        oauth2RedirectUrl: oauth2RedirectUrl,
        deepLinking: true,
    })
</script>

</body>
</html>
"#;

pub(crate) fn create_html(document: &str) -> String {
    SWAGGER_UI_TEMPLATE
        .replace("{:style}", SWAGGER_UI_CSS)
        .replace("{:script}", SWAGGER_UI_JS)
        .replace("{:spec}", document)
}

pub(crate) fn create_endpoint(document: String) -> impl Endpoint {
    let ui_html = create_html(&document);
    poem::Route::new()
        .at("/", create_html_endpoint(ui_html))
        .at(
            "/oauth-receiver.html",
            create_html_endpoint(OAUTH_RECEIVER_HTML),
        )
}
