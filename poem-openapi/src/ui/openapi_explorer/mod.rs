use poem::{endpoint::make_sync, web::Html, Endpoint};

const REDOC_JS: &str = include_str!("openapi-explorer.min.js");

const REDOC_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html>
  <head>
    <title>OpenAPI Explorer</title>
    <!-- needed for adaptive design -->
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <link href="https://fonts.googleapis.com/css?family=Montserrat:300,400,700|Roboto:300,400,700" rel="stylesheet">
    <style type="text/css">
      :root {
        --font-regular: Montserrat;
      }
    </style>

    <script charset="UTF-8">{:script}</script>
  </head>
  <body>
    <openapi-explorer></openapi-explorer>
    
    <script>
        let spec = {:spec};
        document.getElementsByTagName('openapi-explorer')[0].loadSpec(spec).catch(console.error);
    </script>
  </body>
</html>
"#;

pub(crate) fn create_html(document: &str) -> String {
    REDOC_TEMPLATE
        .replace("{:script}", REDOC_JS)
        .replace("{:spec}", document)
}

pub(crate) fn create_endpoint(document: &str) -> impl Endpoint {
    let ui_html = create_html(document);
    poem::Route::new().at("/", make_sync(move |_| Html(ui_html.clone())))
}
