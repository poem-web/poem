use poem::web::Query;
use poem::{handler, IntoResponse};
use serde::Deserialize;

extern "C" {
    fn udf_add(a: i32, b: i32) -> i32;
}

#[derive(Deserialize)]
struct Params {
    a: i32,
    b: i32,
}

#[handler]
async fn index(Query(params): Query<Params>) -> impl IntoResponse {
    format!("{} + {} = {}", params.a, params.b, unsafe {
        udf_add(params.a, params.b)
    })
}

#[no_mangle]
fn start() {
    poem::wasi::run(index);
}
