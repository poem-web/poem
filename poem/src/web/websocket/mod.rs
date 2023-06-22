//! Websocket extractor and response.
//!
//! # Example
//!
//! ```
//! use futures_util::{SinkExt, StreamExt};
//! use poem::{
//!     get, handler,
//!     web::websocket::{Message, WebSocket},
//!     IntoResponse, Route,
//! };
//!
//! #[handler]
//! async fn index(ws: WebSocket) -> impl IntoResponse {
//!     ws.on_upgrade(|mut socket| async move {
//!         if let Some(Ok(Message::Text(text))) = socket.next().await {
//!             let _ = socket.send(Message::Text(text)).await;
//!         }
//!     })
//! }
//!
//! let app = Route::new().at("/", get(index));
//! ```

mod extractor;
mod message;
mod stream;
mod utils;

pub use extractor::{BoxWebSocketUpgraded, WebSocket, WebSocketUpgraded};
pub use message::{CloseCode, Message};
pub use stream::WebSocketStream;

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use futures_util::{SinkExt, StreamExt};
    use http::{header, HeaderValue};

    use super::*;
    use crate::{
        handler,
        listener::{Acceptor, Listener, TcpListener},
        IntoResponse, Server,
    };

    #[tokio::test]
    async fn test_negotiation() {
        #[handler(internal)]
        async fn index(ws: WebSocket) -> impl IntoResponse {
            ws.protocols(["aaa", "bbb"]).on_upgrade(|_| async move {})
        }

        let acceptor = TcpListener::bind("127.0.0.1:0")
            .into_acceptor()
            .await
            .unwrap();
        let addr = acceptor
            .local_addr()
            .remove(0)
            .as_socket_addr()
            .cloned()
            .unwrap();

        let handle = tokio::spawn(async move {
            let _ = Server::new_with_acceptor(acceptor).run(index).await;
        });

        let (_, resp) = tokio_tungstenite::connect_async(format!("ws://{addr}"))
            .await
            .unwrap();
        assert_eq!(resp.headers().get(header::SEC_WEBSOCKET_PROTOCOL), None);

        async fn check(addr: SocketAddr, protocol: &str, value: Option<&HeaderValue>) {
            let (_, resp) = tokio_tungstenite::connect_async(
                http::Request::builder()
                    .uri(format!("ws://{addr}"))
                    .header(header::SEC_WEBSOCKET_PROTOCOL, protocol)
                    .header(header::SEC_WEBSOCKET_KEY, "test_key")
                    .header(header::UPGRADE, "websocket")
                    .header(header::HOST, "localhost")
                    .header(header::CONNECTION, "upgrade")
                    .header(header::SEC_WEBSOCKET_VERSION, "13")
                    .body(())
                    .unwrap(),
            )
            .await
            .unwrap();
            assert_eq!(resp.headers().get(header::SEC_WEBSOCKET_PROTOCOL), value);
        }

        check(addr, "aaa", Some(&HeaderValue::from_static("aaa"))).await;
        check(addr, "bbb", Some(&HeaderValue::from_static("bbb"))).await;
        check(addr, "ccc", None).await;

        handle.abort();
    }

    #[tokio::test]
    async fn test_websocket_echo() {
        #[handler(internal)]
        async fn index(ws: WebSocket) -> impl IntoResponse {
            ws.on_upgrade(|mut stream| async move {
                while let Some(Ok(msg)) = stream.next().await {
                    if let Message::Text(text) = msg {
                        if stream
                            .send(Message::Text(text.to_uppercase()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            })
        }

        let acceptor = TcpListener::bind("127.0.0.1:0")
            .into_acceptor()
            .await
            .unwrap();
        let addr = acceptor
            .local_addr()
            .remove(0)
            .as_socket_addr()
            .cloned()
            .unwrap();
        let server = Server::new_with_acceptor(acceptor);

        let handle = tokio::spawn(async move {
            let _ = server.run(index).await;
        });

        let (mut client_stream, _) = tokio_tungstenite::connect_async(format!("ws://{addr}"))
            .await
            .unwrap();

        client_stream
            .send(tokio_tungstenite::tungstenite::Message::Text(
                "aBc".to_string(),
            ))
            .await
            .unwrap();
        assert_eq!(
            client_stream.next().await.unwrap().unwrap(),
            tokio_tungstenite::tungstenite::Message::Text("ABC".to_string())
        );

        client_stream
            .send(tokio_tungstenite::tungstenite::Message::Text(
                "def".to_string(),
            ))
            .await
            .unwrap();
        assert_eq!(
            client_stream.next().await.unwrap().unwrap(),
            tokio_tungstenite::tungstenite::Message::Text("DEF".to_string())
        );

        handle.abort();
    }
}
