use poem::{handler, post, web::Json, IntoResponse, Route};
use serde::Deserialize;

#[derive(Deserialize)]
struct AddParams {
    a: i32,
    b: i32,
}

#[handler]
async fn index(Json(params): Json<AddParams>) -> impl IntoResponse {
    format!("{} + {} = {}", params.a, params.b, params.a + params.b)
}

#[no_mangle]
fn start() {
    poem::runtime::wasi::run(Route::new().at("/", post(index)));
}
