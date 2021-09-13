use poem::{
    error::ParseJsonError, handler, listener::TcpListener, route, route::post, web::Json, Result,
    Server,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CreateSomething {
    name: String,
}

#[handler]
fn hello(res: Result<Json<CreateSomething>, ParseJsonError>) -> Json<serde_json::Value> {
    let res = match res {
        Ok(Json(req)) => serde_json::json! ({
            "code": 0,
            "message": req.name,
        }),
        Err(err) => serde_json::json! ({
            "code": 1,
            "message": err.to_string()
        }),
    };
    Json(res)
}

// right:
// curl -d '{"name": "Jack"}' http://127.0.0.1:3000/hello
// {"code": 0, "message": "hello: Jack"}
//
// error:
// curl -d '{"badkey": "Jack"}' http://127.0.0.1:3000/hello
// {"code": 1, "message": "missing field `name` at line 1 column 20"}
#[tokio::main]
async fn main() {
    let app = route().at("/hello", post(hello));
    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await.unwrap();
    server.run(app).await.unwrap();
}
