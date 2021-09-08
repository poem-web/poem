//! Websocket extractor and response.
//!
//! # Example
//!
//! ```
//! use futures_util::{SinkExt, StreamExt};
//! use poem::{
//!     handler, route,
//!     route::get,
//!     web::websocket::{Message, WebSocket},
//!     IntoResponse,
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
//! let mut app = route();
//! app.at("/", get(index));
//! ```

mod extractor;
mod message;
mod stream;
mod utils;

pub use extractor::WebSocket;
pub use message::{CloseCode, Message};
pub use stream::WebSocketStream;
