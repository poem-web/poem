use poem::Endpoint;

use super::create_html_endpoint;

const TEMPLATE: &str = include_str!("stoplight-elements.html");

pub(crate) fn create_html(document: &str) -> String {
    TEMPLATE.replace("'{:spec}'", document)
}

pub(crate) fn create_endpoint(document: String) -> impl Endpoint {
    let ui_html = create_html(&document);
    poem::Route::new().at("/", create_html_endpoint(ui_html))
}
