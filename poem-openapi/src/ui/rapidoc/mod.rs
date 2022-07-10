use poem::{endpoint::make_sync, web::Html, Endpoint};

const RAPIDOC_JS: &str = include_str!("rapidoc-min.js");
const OAUTH_RECEIVER_HTML: &str = include_str!("oauth-receiver.html");

const RAPIDOC_TEMPLATE: &str = r#"
<html charset="UTF-8">
<head>
    <meta http-equiv="Content-Type" content="text/html;charset=utf-8">
    <meta name="viewport" content="width=device-width, minimum-scale=1, initial-scale=1, user-scalable=yes">
    <link href="https://fonts.googleapis.com/css2?family=Open+Sans:wght@300;600&family=Roboto+Mono&display=swap" rel="stylesheet">
    <title>RapiDoc</title>
    <script charset="UTF-8">{:script}</script>
</head>
</html>
<body>

    <rapi-doc
        id="thedoc"
        theme="light"
        render-style = "focused"
        show-header	= "false"
        show-components = "true"
        allow-try="true"
        allow-authentication = "true"
        regular-font="Open Sans"
        mono-font = "Roboto Mono"
        font-size = "large"
        schema-description-expanded = "true"	
    >
    </rapi-doc>
    <script>
    document.addEventListener('DOMContentLoaded', (event) => {
        let docEl = document.getElementById("thedoc");
        docEl.loadSpec({:spec});
    })
    </script>

</body>
"#;

pub(crate) fn create_html(document: &str) -> String {
    RAPIDOC_TEMPLATE
        .replace("{:script}", RAPIDOC_JS)
        .replace("{:spec}", document)
}

pub(crate) fn create_endpoint(document: &str) -> impl Endpoint {
    let ui_html = create_html(document);
    let oauth_receiver_html = OAUTH_RECEIVER_HTML.replace("{:script}", RAPIDOC_JS);

    poem::Route::new()
        .at("/", make_sync(move |_| Html(ui_html.clone())))
        .at(
            "/oauth-receiver.html",
            make_sync(move |_| Html(oauth_receiver_html.clone())),
        )
}
