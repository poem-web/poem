# 快速开始

## 添加依赖库

```toml
[dependencies]
poem = "1.0"
serde = "1.0"
tokio = { version = "1.12.0", features = ["rt-multi-thread", "macros"] }
```

## 实现一个Endpoint

`handler` 宏将函数转换为实现了 `Endpoint` 的类型，`Endpoint` trait 表示一种可以处理 HTTP 请求的类型。

这个函数可以接收一个或多个参数，每个参数都是一个提取器，可以从 HTTP 请求中提取你想要的信息。

提取器实现了 `FromRequest` trait，你也可以实现这个 trait 来创建你自己的提取器。

函数的返回值必须是实现了 `IntoResponse` trait 的类型。它可以通过 `IntoResponse::into_response` 方法将自己转化为一个 HTTP 响应。

下面的函数有一个提取器，它从 uri 请求的 query 中提取 `name` 和 `value` 参数并返回一个 `String`，该字符串将被转换为 HTTP 响应。

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

## HTTP 服务器

让我们启动一个服务器，它监听 `127.0.0.1:3000`，请忽略这些 `unwrap` 调用，这只是一个例子。

`Server::run` 函数接受任何实现了 `Endpoint` Trait 的类型。在这个例子中，我们没有路由对象，因此任何请求路径都将由 `index` 函数处理。

```rust

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:3000");
    Server::new(listener).run(index).await.unwrap();
}
```

这样，一个简单的例子就实现了，我们可以运行它，然后使用 `curl` 做一些测试。

```shell
> curl http://localhost:3000?name=a&value=10
name=10
```
