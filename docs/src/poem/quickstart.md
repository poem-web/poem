# Quickstart

## Add dependency libraries

```toml
[dependencies]
poem = "0.4"
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
request uri. And return a `String`, the string will be converted into an HTTP response.

```rust
use serde::Deserialize;
use poem::{handler, web::Query};

#[derive(Deserialize)]
struct Params {
    name: String,
    value: i32,
}

#[handler]
async fn index(Query(params): Query<Params>) -> String {
    format!("{}={}", name, value)
}
```

## Server HTTP server

Let's start a server, it listens to `127.0.0.1:3000`, please ignore these `unwrap` calls, this is just an example.

The `Server::run` function accepts any type that implements the `Endpoint` feature. In this example we don't have a 
routing object, so any request path will be handled by the `index` function.

```rust
use poem::Server;

#[tokio::main]
async fn main() {
    let server = Server::bind("127.0.0.1:3000").await.unwrap();
    server.run(index).await.unwrap();
}
```

In this way, a simple example is implemented, we can run it and then use `curl` to do some tests.

```shell
> curl http://localhost:3000?name=a&value=10
name=10
```
