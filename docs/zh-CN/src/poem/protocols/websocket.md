# Websocket

Websocket 允许在客户端和服务器之间进行双向通信的长连接。

`Poem` 提供了一个 `WebSocket` 提取器来创建这个连接。

当连接升级成功时，调用指定的闭包来发送和接收数据。

下面的例子是一个回显服务，它总是发送接收到的数据。

**注意这个 Endpoint 的输出必须是`WebSocket::on_upgrade`函数的返回值，否则无法正确创建连接。**

```rust
use futures_util::{SinkExt, StreamExt};
use poem::{
    handler, Route, get,
    web::websocket::{Message, WebSocket},
    IntoResponse,
};

#[handler]
async fn index(ws: WebSocket) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        if let Some(Ok(Message::Text(text))) = socket.next().await {
            let _ = socket.send(Message::Text(text)).await;
        }
    })
}

let app = Route::new().at("/", get(index));
```