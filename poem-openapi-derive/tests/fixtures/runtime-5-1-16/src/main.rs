use poem_openapi::{
    OpenApi,
    param::{Header, Path, Query},
};

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/items/:id", method = "get")]
    #[allow(unused_variables)]
    async fn get(
        &self,
        id: Path<i32>,
        headers: Header<Vec<i32>>,
        #[oai(style = "pipe_delimited")] query: Query<Vec<i32>>,
    ) {
    }
}

fn main() {
    let _ = <Api as OpenApi>::meta();
}
