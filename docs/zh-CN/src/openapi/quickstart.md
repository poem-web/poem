# 快速开始

下面这个例子，我们定义了一个路径为`/hello`的API，它接受一个名为`name`的URL参数，并且返回一个字符串作为响应内容。`name`参数的类型是`Option<String>`，意味着这是一个可选参数。

运行以下代码后，用浏览器打开`http://localhost:3000`就能看到`Swagger UI`，你可以用它来浏览API的定义并且测试它们。

```rust
use poem::{listener::TcpListener, Route};
use poem_openapi::{payload::PlainText, OpenApi, OpenApiService};

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/hello", method = "get")]
    async fn index(
        &self,
        #[oai(name = "name", in = "query")] name: Option<String>, // in="query" 说明这个参数来自Url
    ) -> PlainText<String> { // PlainText是响应类型，它表明该API的响应类型是一个字符串，Content-Type是`text/plain`
        match name {
            Some(name) => PlainText(format!("hello, {}!", name)),
            None => PlainText("hello!".to_string()),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // 创建一个TCP监听器
    let listener = TcpListener::bind("127.0.0.1:3000");
  
    // 创建API服务
    let api_service = OpenApiService::new(Api, "Demo", "0.1.0")
        .title("Hello World")
        .server("http://localhost:3000/api");
  
    // 创建Swagger UI端点
    let ui = api_service.swagger_ui();
    
    // 创建OpenApi输出规范的端点
    let spec = api_service.spec_endpoint();

    // 启动服务器，并指定api的根路径为 /api，Swagger UI的路径为 /
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

这是`poem-openapi`的一个例子，所以你也可以直接执行以下命令来验证：

```shell
git clone https://github.com/poem-web/poem
cargo run --bin example-openapi-hello-world
```
