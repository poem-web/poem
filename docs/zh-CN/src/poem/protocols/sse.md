# 服务器发送的事件 (SSE)

SSE 允许服务器不断地向客户端推送数据。

你需要使用实现 `Stream<Item=Event>` 的类型创建一个 `SSE` 响应。

下面示例中的端点将发送三个事件。

```rust
use futures_util::stream;
use poem::{
    handler, Route, get,
    http::StatusCode,
    web::sse::{Event, SSE},
    Endpoint, Request,
};

#[handler]
fn index() -> SSE {
    SSE::new(stream::iter(vec![
        Event::message("a"),
        Event::message("b"),
        Event::message("c"),
    ]))
}

let app = Route::new().at("/", get(index));
```
