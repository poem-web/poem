# 中间件

中间件可以在处理请求之前或之后做一些事情。

`Poem` 提供了一些常用的中间件实现。

- `AddData`

    用于将状态附加到请求，例如用于身份验证的 token。

- `SetHeader`

    用于向响应添加一些特定的 HTTP 标头。

- `Cors`

    用于 CORS 跨域资源共享。

- `Tracing`

  使用 [`tracing`](https://crates.io/crates/tracing) 记录所有请求和响应。

- `Compression`

  用于解压请求体和压缩响应体。

## 自定义中间件

实现你自己的中间件很容易，你只需要实现 `Middleware` trait，它是一个转换器
将输入 Endpoint 转换为另一个 Endpoint。

以下示例创建一个自定义中间件，该中间件读取名为“X-Token”的 HTTP 请求标头的值和将其添加为请求的状态。

```rust
use poem::{handler, web::Data, Endpoint, EndpointExt, Middleware, Request};

/// 从 HTTP 标头中提取 token 的中间件。
struct TokenMiddleware;

impl<E: Endpoint> Middleware<E> for TokenMiddleware {
    type Output = TokenMiddlewareImpl<E>;
  
    fn transform(&self, ep: E) -> Self::Output {
        TokenMiddlewareImpl { ep }
    }
}

/// TokenMiddleware 生成的新 Endpoint 类型。
struct TokenMiddlewareImpl<E> {
    ep: E,
}

const TOKEN_HEADER: &str = "X-Token";

/// Token 数据
struct Token(String);

#[poem::async_trait]
impl<E: Endpoint> Endpoint for TokenMiddlewareImpl<E> {
    type Output = E::Output;
  
    async fn call(&self, mut req: Request) -> Self::Output {
        if let Some(value) = req
            .headers()
            .get(TOKEN_HEADER)
            .and_then(|value| value.to_str().ok())
        {
            // 将 token 数据插入到请求的扩展中。
            let token = value.to_string();
            req.extensions_mut().insert(Token(token));
        }
      
        // 调用内部 endpoint。
        self.ep.call(req).await
    }
}

#[handler]
async fn index(Data(token): Data<&Token>) -> String {
    token.0.clone()
}

// 使用 `TokenMiddleware` 中间件转换 `index` endpoint。
let ep = index.with(TokenMiddleware);
```

## 带函数的自定义中间件

您还可以使用函数来实现中间件。

```rust
async fn extract_token<E: Endpoint>(next: E, mut req: Request) -> Response {
    if let Some(value) = req
        .headers()
        .get(TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
    {
        // 将 token 数据插入到请求的扩展中。
        let token = value.to_string();
        req.extensions_mut().insert(Token(token));
    }

    // 调用下一个 endpoint。
    next.call(req).await
}

#[handler]
async fn index(Data(token): Data<&Token>) -> String {
  token.0.clone()
}

let ep = index.around(extract_token);
```
