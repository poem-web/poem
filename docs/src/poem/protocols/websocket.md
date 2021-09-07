# Websocket

Websocket allows a long connection for two-way communication between the client and the server.

`Poem` provides a `WebSocket` extractor to create this connection.

When the connection is successfully upgraded, a specified closure is called to send and receive data.

The following example is an echo service, which always sends out the received data.

**Note that the output of this endpoint must be the return value of the `WebSocket::on_upgrade` function, otherwise the 
connection cannot be created correctly.**

```rust
use futures_util::{SinkExt, StreamExt};
use poem::{
    handler, route,
    route::get,
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
```