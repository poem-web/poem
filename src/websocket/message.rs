/// Status code used to indicate why an endpoint is closing the WebSocket connection.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CloseCode {
    /// Indicates a normal closure, meaning that the purpose for
    /// which the connection was established has been fulfilled.
    Normal,
    /// Indicates that an endpoint is "going away", such as a server
    /// going down or a browser having navigated away from a page.
    Away,
    /// Indicates that an endpoint is terminating the connection due
    /// to a protocol error.
    Protocol,
    /// Indicates that an endpoint is terminating the connection
    /// because it has received a type of data it cannot accept (e.g., an
    /// endpoint that understands only text data MAY send this if it
    /// receives a binary message).
    Unsupported,
    /// Indicates that no status code was included in a closing frame.
    Status,
    /// Indicates an abnormal closure.
    Abnormal,
    /// Indicates that an endpoint is terminating the connection
    /// because it has received data within a message that was not
    /// consistent with the type of the message (e.g., non-UTF-8 \[RFC3629\]
    /// data within a text message).
    Invalid,
    /// Indicates that an endpoint is terminating the connection
    /// because it has received a message that violates its policy.  This
    /// is a generic status code that can be returned when there is no
    /// other more suitable status code (e.g., Unsupported or Size) or if there
    /// is a need to hide specific details about the policy.
    Policy,
    /// Indicates that an endpoint is terminating the connection
    /// because it has received a message that is too big for it to
    /// process.
    Size,
    /// Indicates that an endpoint (client) is terminating the
    /// connection because it has expected the server to negotiate one or
    /// more extension, but the server didn't return them in the response
    /// message of the WebSocket handshake.  The list of extensions that
    /// are needed should be given as the reason for closing.
    /// Note that this status code is not used by the server, because it
    /// can fail the WebSocket handshake instead.
    Extension,
    /// Indicates that a server is terminating the connection because
    /// it encountered an unexpected condition that prevented it from
    /// fulfilling the request.
    Error,
    /// Indicates that the server is restarting. A client may choose to reconnect,
    /// and if it does, it should use a randomized delay of 5-30 seconds between attempts.
    Restart,
    /// Indicates that the server is overloaded and the client should either connect
    /// to a different IP (when multiple targets exist), or reconnect to the same IP
    /// when a user has performed an action.
    Again,
    /// Code reserved for the future.
    Reserved(u16),
}

impl From<u16> for CloseCode {
    fn from(code: u16) -> Self {
        use CloseCode::*;

        match code {
            1000 => Normal,
            1001 => Away,
            1002 => Protocol,
            1003 => Unsupported,
            1005 => Status,
            1006 => Abnormal,
            1007 => Invalid,
            1008 => Policy,
            1009 => Size,
            1010 => Extension,
            1011 => Error,
            1012 => Restart,
            1013 => Again,
            _ => Reserved(code),
        }
    }
}

impl From<CloseCode> for u16 {
    fn from(code: CloseCode) -> Self {
        use CloseCode::*;

        match code {
            Normal => 1000,
            Away => 1001,
            Protocol => 1002,
            Unsupported => 1003,
            Status => 1005,
            Abnormal => 1006,
            Invalid => 1007,
            Policy => 1008,
            Size => 1009,
            Extension => 1010,
            Error => 1011,
            Restart => 1012,
            Again => 1013,
            Reserved(code) => code,
        }
    }
}

/// An enum representing the various forms of a WebSocket message.
pub enum Message {
    /// A text WebSocket message
    Text(String),

    /// A binary WebSocket message
    Binary(Vec<u8>),

    /// A ping message with the specified payload
    ///
    /// The payload here must have a length less than 125 bytes
    Ping(Vec<u8>),

    /// A pong message with the specified payload
    ///
    /// The payload here must have a length less than 125 bytes
    Pong(Vec<u8>),

    /// A close message with the optional close frame.
    Close(Option<(CloseCode, String)>),
}
