use derive_more::Display;
use poem::{post, web::Path, Server, http::StatusCode, route, Response, ResponseError, EndpointExt, web::Json};
use serde::Serialize;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct JsonType1 {
    name: String
}

#[post]
fn hello(Json(json1): Json<JsonType1>) -> String {
    format!(r#"{{"code": 0, "message": "{}"}}"#, json1.name)
}

/*
    right:
    curl -d '{"name": "Jack"}' http://127.0.0.1:3000/hello
    {"code": 0, "message": "hello: Jack"}

    error:
    curl -d '{"badkey": "Jack"}' http://127.0.0.1:3000/hello
    {"code": 1, "message": "missing field `name` at line 1 column 20"}
 */
#[tokio::main]
async fn main() {
    let app = route().at("/hello", hello).map_to_response()
        .map(|res| async move {
            match res {
                Ok(mut resp) if resp.status() == StatusCode::BAD_REQUEST || resp.status() == StatusCode::INTERNAL_SERVER_ERROR  => {
                    let body = resp.take_body().into_string().await.unwrap();
                    let s = format!(r#"{{"code": 1, "message": "{}"}}"#, body);
                    Ok(Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(s))
                },
                Ok(resp) => Ok(resp),
                Err(_) => unreachable!(),
            }
        });;
    let server = Server::bind("127.0.0.1:3000").await.unwrap();
    server.run(app).await.unwrap();
}
