# 提取器

提取器用于从 HTTP 请求中提取某些内容。

`Poem` 提供了一些常用的提取器来从 HTTP 请求中提取一些东西。

你可以使用一个或多个提取器作为函数的参数，最多 16 个。

在下面的例子中，`index` 函数使用 3 个提取器来提取远程地址、HTTP 方法和 URI。

```rust
#[handler]
fn index(remote_addr: SocketAddr, method: Method, uri: &Uri) {}
```

# 内置提取器

 - **Option&lt;T>**

    从传入的请求中提取 `T`，如果失败就返回 `None`。

 - **&Request**

    从传入的请求中提取 `Request`.

 - **&RemoteAddr**

    从请求中提取远端对等地址 [`RemoteAddr`]。

 - **&LocalAddr**

    从请求中提取本地服务器的地址 [`LocalAddr`]。

 - **Method**

    从传入的请求中提取 `Method`。

 - **Version**

    从传入的请求中提取 `Version`。

 - **&Uri**

    从传入的请求中提取 `Uri`。

 - **&HeaderMap**

    从传入的请求中提取 `HeaderMap`。

 - **Data&lt;&T>**

    从传入的请求中提取 `Data` 。

 - **TypedHeader&lt;T>**

    从传入的请求中提取 `TypedHeader`。

 - **Path&lt;T>**

    从传入的请求中提取 `Path`。

 - **Query&lt;T>**

    从传入的请求中提取 `Query`。

 - **Form&lt;T>**

    从传入的请求中提取 `Form`。

 - **Json&lt;T>**

    从传入的请求中提取 `Json` 。

    _这个提取器将接管请求的主体，所以你应该避免在一个处理程序中使用多个这种类型的提取器。_

 - **TempFile**

    从传入的请求中提取 `TempFile`。

    _这个提取器将接管请求的主体，所以你应该避免在一个处理程序中使用多个这种类型的提取器。_

 - **Multipart**

    从传入的请求中提取 `Multipart`。

    _这个提取器将接管请求的主体，所以你应该避免在一个处理程序中使用多个这种类型的提取器。_

 - **&CookieJar**

    从传入的请求中提取 `CookieJar`](cookie::CookieJar)。

    _需要 `CookieJarManager` 中间件。_

 - **&Session**

    从传入的请求中提取 [`Session`](crate::session::Session)。

    _需要 `CookieSession` 或 `RedisSession` 中间件。_

 - **Body**

     从传入的请求中提取 `Body`。

     _这个提取器将接管请求的主体，所以你应该避免在一个处理程序中使用多个这种类型的提取器。_

 - **String**

    从传入的请求中提取 body 并将其解析为 utf8 字符串。

    _这个提取器将接管请求的主体，所以你应该避免在一个处理程序中使用多个这种类型的提取器。_

 - **Vec&lt;u8>**

    从传入的请求中提取 body 并将其收集到 `Vec<u8>`.

    _这个提取器将接管请求的主体，所以你应该避免在一个处理程序中使用多个这种类型的提取器。_

 - **Bytes**

    从传入的请求中提取 body 并将其收集到 `Bytes`.

    _这个提取器将接管请求的主体，所以你应该避免在一个处理程序中使用多个这种类型的提取器。_

 - **WebSocket**

    准备接受 websocket 连接。

## 处理提取器错误

默认情况下，当发生错误时，提取器会返回`400 Bad Request`，但有时您可能想要更改这种行为，因此您可以自己处理错误。

在下面的例子中，当 `Query` 提取器失败时，它将返回一个 `500 Internal Server Error` 响应以及错误原因。

```rust
use poem::web::Query;
use poem::error::ParseQueryError;
use poem::{IntoResponse, Response};
use poem::http::StatusCode;

#[derive(Debug, Deserialize)]
struct Params {
    name: String,
}

#[handler]
fn index(res: Result<Query<Params>, ParseQueryError>) -> Response {
    match res {
        Ok(Query(params)) => params.name.into_response(),
        Err(err) => Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(err.to_string()),
    }
}
```

## 自定义提取器

您还可以实现自己的提取器。

以下是自定义 token 提取器的示例，它提取来自 `MyToken` 标头的 token。
 
```rust
use poem::{
    get, handler, http::StatusCode, listener::TcpListener, FromRequest, Request,
    RequestBody, Response, Route, Server,
};

struct Token(String);

// Token 提取器的错误类型
#[derive(Debug)]
struct MissingToken;

/// 自定义错误也可以重用
impl IntoResponse for MissingToken {
    fn into_response(self) -> Response {
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("missing token")
    }
}

// 实现一个 token 提取器
#[poem::async_trait]
impl<'a> FromRequest<'a> for Token {
    type Error = MissingToken;

    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self, Self::Error> {
        let token = req
            .headers()
            .get("MyToken")
            .and_then(|value| value.to_str().ok())
            .ok_or(MissingToken)?;
        Ok(Token(token.to_string()))
    }
}

#[handler]
async fn index(token: Token) {
    assert_eq!(token.0, "token123");
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug");
    }
    tracing_subscriber::fmt::init();

    let app = Route::new().at("/", get(index));
    let listener = TcpListener::bind("127.0.0.1:3000");
    let server = Server::new(listener).await?;
    server.run(app).await
}
```