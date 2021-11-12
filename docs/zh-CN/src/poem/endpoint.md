# Endpoint

Endpoint 可以处理 HTTP 请求。您可以实现`Endpoint` trait 来创建您自己的Endpoint。  
`Poem` 还提供了一些方便的功能来轻松创建自定义 Endpoint 类型。

在上一章中，我们学习了如何使用 `handler` 宏将函数转换为 Endpoint。

现在让我们看看如何通过实现 `Endpoint` trait 来创建自己的 Endpoint。

这是 `Endpoint` trait 的定义，你需要指定 `Output` 的类型并实现 `call` 方法。

```rust
/// 一个 HTTP 请求处理程序。
#[async_trait]
pub trait Endpoint: Send + Sync + 'static {
    /// 代表 endpoint 的响应。
    type Output: IntoResponse;

    /// 获取对请求的响应。
    async fn call(&self, req: Request) -> Self::Output;
}
```

现在我们实现一个 `Endpoint`，它接收 HTTP 请求并输出一个包含请求方法和路径的字符串。

`Output` 关联类型必须是实现 `IntoResponse` trait 的类型。Poem 已为大多数常用类型实现了它。

由于 `Endpoint` 包含一个异步方法 `call`，我们需要用 `async_trait` 宏来修饰它。

```rust
struct MyEndpoint;

#[async_trait]
impl Endpoint for MyEndpoint {
    type Output = String;
    
    async fn call(&self, req: Request) -> Self::Output {
        format!("method={} path={}", req.method(), req.uri().path());
    }
}
```

## 从函数创建

你可以使用 `poem::endpoint::make` 和 `poem::endpoint::make_sync` 从异步函数和同步函数创建 Endpoint。

以下 Endpoint 执行相同的操作：

```rust
let ep = poem::endpoint::make(|req| async move {
    format!("method={} path={}", req.method(), req.uri().path())
});
```

## EndpointExt

`EndpointExt` trait 提供了一些方便的函数来转换 Endpoint 的输入或输出。

- `EndpointExt::before` 用于转换请求。
- `EndpointExt::after` 用于转换输出。
- `EndpointExt::map_ok`、`EndpointExt::map_err`、`EndpointExt::and_then` 用于处理 `Result<T>` 类型的输出。

## 使用 Result 类型

`Poem` 还为 `poem::Result<T>` 类型实现了 `IntoResponse`，因此它也可以用作 Endpoint，因此你可以在 `call` 方法中使用 `?`。

```rust
struct MyEndpoint;

#[async_trait]
impl Endpoint for MyEndpoint {
    type Output = poem::Result<String>;
    
    async fn call(&self, req: Request) -> Self::Output {
        Ok(req.take_body().into_string().await?)
    }
}
```

你可以使用 `EndpointExt::map_to_response` 方法将 Endpoint 的输出转换为 `Response` 类型，或者使用 `EndpointExt::map_to_result` 将输出转换为 `poem::Result<Response>` 类型。

```rust
let ep = MyEndpoint.map_to_response() // impl Endpoint<Output = Response>
```
