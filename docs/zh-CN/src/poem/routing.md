# 路由

路由对象用于将指定路径和方法的请求分派到指定 Endpoint。

路由对象实际上是一个 Endpoint，它实现了 Endpoint 特性。

在下面的例子中，我们将 `/a` 和 `/b` 的请求分派到不同的 Endpoint。

```rust
use poem::{handler, Route};

#[handler]
async fn a() -> &'static str { "a" }

#[handler]
async fn b() -> &'static str { "b" }

let ep = Route::new()
    .at("/a", a)
    .at("/b", b);
```

## 捕获变量

使用`:NAME`捕获路径中指定段的值，或者使用`*NAME`捕获路径中的所有指定前缀的值。

在下面的示例中，捕获的值将存储在变量 `value` 中，你可以使用路径提取器来获取它们。

```rust
#[handler]
async fn a(Path(String): Path<String>) {} 

let ep = Route::new()
    .at("/a/:value/b", handler)
    .at("/prefix/*value", handler);
```

## 正则表达式

可以使用正则表达式进行匹配，`<REGEX>` 或`:NAME<REGEX>`，第二个可以将匹配的值捕获到一个变量中。

```rust
let ep = Route::new()
    .at("/a/<\\d+>", handler)
    .at("/b/:value<\\d+>", handler);
```

## 嵌套

有时我们想为指定的 Endpoint 分配一个带有指定前缀的路径，以便创建一些功能独立的组件。

在下面的例子中，`hello` Endpoint 的请求路径是 `/api/hello`。

```rust
let api = Route::new().at("/hello", hello);
let ep = api.nest("/api", api);
```

静态文件服务就是这样一个独立的组件。

```rust
let ep = Route::new().nest("/files", Files::new("./static_files"));
```

## 方法路由

上面介绍的路由对象只能通过一些指定的路径进行调度，但是通过路径和方法进行调度更常见。 `Poem` 提供了另一个路由对象 `RouteMethod`，当它与 `Route` 对象结合时，它可以提供这种能力。

`Poem` 提供了一些方便的函数来创建 `RouteMethod` 对象，它们都以 HTTP 标准方法命名。

```rust
use poem::{Route, get, post};

let ep = Route::new()
    .at("/users", get(get_user).post(create_user).delete(delete_user).put(update_user));
```
