use poem::Endpoint;

use super::create_html_endpoint;

const SCALAR_JS: &str = include_str!("scalar.min.js");

const SCALAR_TEMPLATE: &str = r#"
<!doctype html>
<html>
  <head>
    <title>Scalar</title>
    <meta charset="utf-8" />
    <meta
      name="viewport"
      content="width=device-width, initial-scale=1" />
    <style>
      body {
        margin: 0;
      }
    </style>

  </head>
  <body>
    <script
      id="api-reference"
      type="application/json"
    >
      {:spec}
    </script>
    <script charset="UTF-8">{:script}</script>
  </body>
</html>
"#;

pub(crate) fn create_html(document: &str) -> String {
    SCALAR_TEMPLATE
        .replace("{:script}", SCALAR_JS)
        .replace("{:spec}", document)
}

pub(crate) fn create_endpoint(document: String) -> impl Endpoint + 'static {
    let ui_html = create_html(&document);
    poem::Route::new().at("/", create_html_endpoint(ui_html))
}
