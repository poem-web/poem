use poem::{handler, web::Query, IntoResponse, Route};
use serde::Deserialize;

extern "C" {
    fn udf_add(a: i32, b: i32) -> i32;

    fn udf_touppercase(buf: u32, buf_len: u32);
}

#[derive(Deserialize)]
struct AddParams {
    a: i32,
    b: i32,
}

#[handler]
async fn add(Query(params): Query<AddParams>) -> impl IntoResponse {
    format!("{} + {} = {}", params.a, params.b, unsafe {
        udf_add(params.a, params.b)
    })
}

#[derive(Deserialize)]
struct ToUppercaseParams {
    value: String,
}

#[handler]
async fn touppercase(Query(mut params): Query<ToUppercaseParams>) -> impl IntoResponse {
    unsafe { udf_touppercase(params.value.as_mut_ptr() as u32, params.value.len() as u32) };
    params.value
}

#[no_mangle]
fn start() {
    poem::wasi::run(Route::new().at("/add", add).at("/uppercase", touppercase));
}
