use poem::{listener::TcpListener, Endpoint, EndpointExt, Route, Server};
use poem_openapi::{param::Query, payload::PlainText, OpenApi, OpenApiService, OperationId};

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/hello", method = "get", operation_id = "index-get")]
    async fn index(&self, name: Query<Option<String>>) -> PlainText<String> {
        match name.0 {
            Some(name) => PlainText(format!("hello, {name}!")),
            None => PlainText("hello!".to_string()),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let api_service =
        OpenApiService::new(Api, "Hello World", "1.0").server("http://localhost:3000/api");
    let ui = api_service.swagger_ui();
    let app = Route::new()
        .nest("/api", api_service)
        .nest("/", ui)
        .around(|ep, req| async move {
            let uri = req.uri().clone();
            let resp = ep.get_response(req).await;

            if let Some(operation_id) = resp.data::<OperationId>() {
                println!("[{}]{} {}", operation_id, uri, resp.status());
            } else {
                println!("{} {}", uri, resp.status());
            }

            Ok(resp)
        });

    Server::new(TcpListener::bind("127.0.0.1:3000"))
        .run(app)
        .await
}
