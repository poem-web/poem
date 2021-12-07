# Quickstart

In the following example, we define an API with a path of `/hello`, which accepts a URL parameter named `name` and returns 
a string as the response content. The type of the `name` parameter is `Option<String>`, which means it is an optional parameter.

Running the following code, open `http://localhost:3000` with a browser to see `Swagger UI`, you can use it to browse API
definitions and test them.

```rust
use poem::{listener::TcpListener, Route};
use poem_openapi::{payload::PlainText, OpenApi, OpenApiService};

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/hello", method = "get")]
    async fn index(
        &self,
        #[oai(name = "name", in = "query")] name: Option<String>, // in="query" means this parameter is parsed from Url
    ) -> PlainText<String> { // PlainText is the response type, which means that the response type of the API is a string, and the Content-Type is `text/plain`
        match name {
            Some(name) => PlainText(format!("hello, {}!", name)),
            None => PlainText("hello!".to_string()),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Create a TCP listener
    let listener = TcpListener::bind("127.0.0.1:3000");
  
    // Create API service
    let api_service = OpenApiService::new(Api, "Demo", "0.1.0")
        .title("Hello World")
        .server("http://localhost:3000/api");
  
    // Enable the Swagger UI
    let ui = api_service.swagger_ui();
    
    // Enable the OpenAPI specification
    let spec = api_service.spec_endpoint();

    // Start the server and specify that the root path of the API is /api, and the path of Swagger UI is /
    poem::Server::new(listener)
        .await?
        .run(
            Route::new()
            .at("/openapi.json", spec)
            .nest("/api", api_service)
            .nest("/", ui)
        )
        .await
}
```

This is an example of `poem-openapi`, so you can also directly execute the following command to play:

```shell
git clone https://github.com/poem-web/poem
cargo run --bin example-openapi-hello-world
```
