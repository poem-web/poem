use std::convert::TryInto;
use std::io::{Error as IoError, ErrorKind};

use sha1::Sha1;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;

use super::{CloseCode, Message};
use crate::HeaderValue;

pub(crate) fn sign(key: &[u8]) -> HeaderValue {
    let mut sha1 = Sha1::default();
    sha1.update(key);
    sha1.update(&b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11"[..]);
    base64::encode(sha1.digest().bytes()).try_into().unwrap()
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

pub(crate) fn tungstenite_code_to_code(
    code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode,
) -> CloseCode {
    let code: u16 = code.into();
    code.into()
}

pub(crate) fn code_to_tungstenite_code(
    code: CloseCode,
) -> tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode {
    let code: u16 = code.into();
    code.into()
}

pub(crate) fn tungstenite_msg_to_message(msg: tokio_tungstenite::tungstenite::Message) -> Message {
    use tokio_tungstenite::tungstenite::Message::*;

    match msg {
        Text(data) => Message::Text(data),
        Binary(data) => Message::Binary(data),
        Ping(data) => Message::Ping(data),
        Pong(data) => Message::Pong(data),
        Close(cf) => {
            Message::Close(cf.map(|cf| (tungstenite_code_to_code(cf.code), cf.reason.to_string())))
        }
    }
}

pub(crate) fn msg_to_tungstenite_message(msg: Message) -> tokio_tungstenite::tungstenite::Message {
    use tokio_tungstenite::tungstenite::Message::*;

    match msg {
        Message::Text(data) => Text(data),
        Message::Binary(data) => Binary(data),
        Message::Ping(data) => Ping(data),
        Message::Pong(data) => Pong(data),
        Message::Close(cf) => Close(cf.map(|(code, reason)| CloseFrame {
            code: code_to_tungstenite_code(code),
            reason: reason.into(),
        })),
    }
}
