use std::io::{Error as IoError, ErrorKind};

use sha1::{Digest, Sha1};
use tokio_tungstenite::tungstenite::protocol::CloseFrame;

use super::{CloseCode, Message};
use crate::http::header::HeaderValue;

pub(crate) fn sign(key: &[u8]) -> HeaderValue {
    let mut sha1 = Sha1::new();
    sha1.update(key);
    sha1.update(&b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11"[..]);
    base64::encode(sha1.finalize()).try_into().unwrap()
}

pub(crate) fn tungstenite_error_to_io_error(
    error: tokio_tungstenite::tungstenite::Error,
) -> IoError {
    use tokio_tungstenite::tungstenite::Error::*;
    match error {
        Io(err) => err,
        _ => IoError::new(ErrorKind::Other, error.to_string()),
    }
}

#[doc(hidden)]
impl From<tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode> for CloseCode {
    fn from(code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode) -> Self {
        let code: u16 = code.into();
        code.into()
    }
}

#[doc(hidden)]
impl From<CloseCode> for tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode {
    fn from(code: CloseCode) -> Self {
        let code: u16 = code.into();
        code.into()
    }
}

#[doc(hidden)]
impl From<tokio_tungstenite::tungstenite::Message> for Message {
    fn from(msg: tokio_tungstenite::tungstenite::Message) -> Self {
        use tokio_tungstenite::tungstenite::Message::*;

        match msg {
            Text(data) => Message::Text(data),
            Binary(data) => Message::Binary(data),
            Ping(data) => Message::Ping(data),
            Pong(data) => Message::Pong(data),
            Frame(_) => unimplemented!("frame is not supported"),
            Close(cf) => Message::Close(cf.map(|cf| (cf.code.into(), cf.reason.to_string()))),
        }
    }
}

#[doc(hidden)]
impl From<Message> for tokio_tungstenite::tungstenite::Message {
    fn from(msg: Message) -> Self {
        use tokio_tungstenite::tungstenite::Message::*;

        match msg {
            Message::Text(data) => Text(data),
            Message::Binary(data) => Binary(data),
            Message::Ping(data) => Ping(data),
            Message::Pong(data) => Pong(data),
            Message::Close(cf) => Close(cf.map(|(code, reason)| CloseFrame {
                code: code.into(),
                reason: reason.into(),
            })),
        }
    }
}
