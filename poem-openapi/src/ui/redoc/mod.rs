use poem::{endpoint::make_sync, web::Html, Endpoint};

const REDOC_JS: &str = include_str!("redoc.standalone.js");

const REDOC_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html>
  <head>
    <title>Redoc</title>
    <!-- needed for adaptive design -->
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <link href="https://fonts.googleapis.com/css?family=Montserrat:300,400,700|Roboto:300,400,700" rel="stylesheet">

    <!--
    Redoc doesn't change outer page styles
    -->
    <style>
      body {
        margin: 0;
        padding: 0;
      }
    </style>
    <script charset="UTF-8">{:script}</script>
  </head>
  <body>
    <div id="redoc-container"></div>
    
    <script>
        let spec = {:spec};
        Redoc.init(spec, {
          scrollYOffset: 50
        }, document.getElementById('redoc-container'));
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
