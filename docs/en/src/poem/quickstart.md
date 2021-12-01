# Quickstart

## Add dependency libraries

```toml
[dependencies]
poem = "1.0"
serde = "1.0"
tokio = { version = "1.12.0", features = ["rt-multi-thread", "macros"] }
```

## Write a endpoint

The `handler` macro converts a function into a type that implements `Endpoint`, and the `Endpoint` trait represents
a type that can handle HTTP requests.

This function can receive one or more parameters, and each parameter is an extractor that can extract something from
the HTTP request.

The extractor implements the `FromRequest` trait, and you can also implement this trait to create your own extractor.

The return value of the function must be a type that implements the `IntoResponse` trait. It can convert itself into an
HTTP response through the `IntoResponse::into_response` method.

The following function has an extractor, which extracts the `name` and `value` parameters from the query string of the 
request uri and return a `String`, the string will be converted into an HTTP response.

```rust
use serde::Deserialize;
use poem::{handler, listener::TcpListener, web::Query, Server};

#[derive(Deserialize)]
struct Params {
    name: String,
    value: i32,
}

#[handler]
async fn index(Query(Params { name, value }): Query<Params>) -> String {
    format!("{}={}", name, value)
}
```

## HTTP server

Let's start a server, it listens to `127.0.0.1:3000`, please ignore these `unwrap` calls, this is just an example.

The `Server::run` function accepts any type that implements the `Endpoint` trait. In this example we don't have a 
routing object, so any request path will be handled by the `index` function.

```rust

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:3000");
    Server::new(listener).run(index).await.unwrap();
}
```

In this way, a simple example is implemented, we can run it and then use `curl` to do some tests.

```shell
> curl http://localhost:3000?name=a&value=10
name=10
```
