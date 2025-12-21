use poem::{Endpoint, endpoint::make_sync, web::Html};

const TEMPLATE: &str = include_str!("stoplight-elements.html");

pub(crate) fn create_html(title: &str, document: &str) -> String {
    TEMPLATE
        .replace("{:title}", title)
        .replace("'{:spec}'", document)
}

pub(crate) fn create_endpoint(title: String, document: String) -> impl Endpoint {
    let ui_html = create_html(&title, &document);
    poem::Route::new().at("/", make_sync(move |_| Html(ui_html.clone())))
}
