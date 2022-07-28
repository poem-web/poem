use std::fmt::Display;

use hyper::{header::HeaderValue, HeaderMap};
use percent_encoding::{percent_decode_str, percent_encode, AsciiSet, CONTROLS};

use crate::Metadata;

const GRPC_STATUS_HEADER_CODE: &str = "grpc-status";
const GRPC_STATUS_MESSAGE_HEADER: &str = "grpc-message";

const ENCODING_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'?')
    .add(b'{')
    .add(b'}');

/// GRPC status code
///
/// Reference: <https://github.com/grpc/grpc/blob/master/doc/statuscodes.md#status-codes-and-their-use-in-grpc>
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Code {
    /// Not an error.
    Ok,

    /// The operation was cancelled, typically by the caller.
    Cancelled,

    /// Unknown error.
    ///
    /// For example, this error may be returned when a Status value received
    /// from another address space belongs to an error space that is not known
    /// in this address space. Also errors raised by APIs that do not return
    /// enough error information may be converted to this error.
    Unknown,

    /// The client specified an invalid argument.
    ///
    /// Note that this differs from `FailedPrecondition`. `InvalidArgument`
    /// indicates arguments that are problematic regardless of the state of the
    /// system (e.g., a malformed file name).
    InvalidArgument,

    /// The deadline expired before the operation could complete.
    ///
    /// For operations that change the state of the system, this error may be
    /// returned even if the operation has completed successfully. For
    /// example, a successful response from a server could have been delayed
    /// long.
    DeadlineExceeded,

    /// Some requested entity
    ///
    /// (e.g., file or directory) was not found. Note to server developers: if a
    /// request is denied for an entire class of users, such as gradual feature
    /// rollout or undocumented allowlist, `NotFound` may be used. If a request
    /// is denied for some users within a class of users, such as user-based
    /// access control, PermissionDenied must be used.
    NotFound,

    /// The entity that a client attempted to create (e.g., file or directory)
    /// already exists.
    AlreadyExists,

    /// The caller does not have permission to execute the specified operation.
    ///
    /// `PermissionDenied` must not be used for rejections caused by exhausting
    /// some resource (use `ResourceExhausted` instead for those errors).
    /// PermissionDenied must not be used if the caller can not be identified
    /// (use `Unauthenticated` instead for those errors). This error code does
    /// not imply the request is valid or the requested entity exists or
    /// satisfies other pre-conditions.
    PermissionDenied,

    /// Some resource has been exhausted, perhaps a per-user quota, or perhaps
    /// the entire file system is out of space.
    ResourceExhausted,

    /// The operation was rejected because the system is not in a state required
    /// for the operation's execution.
    ///
    /// For example, the directory to be deleted is non-empty, an rmdir
    /// operation is applied to a non-directory, etc. Service implementors can
    /// use the following guidelines to decide between `FailedPrecondition`,
    /// `Aborted`, and `Unavailable`:
    ///
    ///  (a) Use `Unavailable` if the client can retry just the failing call.
    ///
    ///  (b) Use `Aborted` if the client should retry at a higher level
    ///      (e.g., when a client-specified test-and-set fails, indicating
    ///      the client should restart a read-modify-write sequence).
    ///
    ///  (c) Use `FailedPrecondition` if the client should not retry until
    ///      the system state has been explicitly fixed. E.g., if an
    ///      "rmdir" fails because the directory is non-empty,
    ///      `FailedPrecondition` should be returned since the client
    ///      should not retry unless the files are deleted from the
    ///      directory.
    FailedPrecondition,

    /// The operation was aborted, typically due to a concurrency issue such as
    /// a sequencer check failure or transaction abort. See the guidelines above
    /// for deciding between `FailedPrecondition`, `Aborted`, and `Unavailable`.
    Aborted,

    /// The operation was attempted past the valid range. E.g., seeking or
    /// reading past end-of-file.
    ///
    /// Unlike `InvalidArgument`, this error indicates
    /// a problem that may be fixed if the system state changes. For example, a
    /// 32-bit file system will generate `InvalidArgument` if asked to read at
    /// an offset that is not in the range [0,2^32-1], but it will generate
    /// `OutOfRange` if asked to read from an offset past the current file size.
    /// There is a fair bit of overlap between `FailedPrecondition` and
    /// `OutOfRange`. We recommend using `OutOfRange` (the more specific error)
    /// when it applies so that callers who are iterating through a space can
    /// easily look for an `OutOfRange` error to detect when they are done.
    OutOfRange,

    /// The operation is not implemented or is not supported/enabled in this
    /// service.
    Unimplemented,

    /// Internal errors.
    ///
    /// This means that some invariants expected by the underlying system have
    /// been broken. This error code is reserved for serious errors.
    Internal,

    /// The service is currently unavailable.
    ///
    /// This is most likely a transient condition, which can be corrected by
    /// retrying with a backoff. Note that it is not always safe to retry
    /// non-idempotent operations.
    Unavailable,

    /// Unrecoverable data loss or corruption.
    DataLoss,

    /// The request does not have valid authentication credentials for the
    /// operation.
    Unauthenticated,

    /// Other codes
    Other(u16),
}

impl From<u16> for Code {
    #[inline]
    fn from(value: u16) -> Self {
        match value {
            0 => Code::Ok,
            1 => Code::Cancelled,
            2 => Code::Unknown,
            3 => Code::InvalidArgument,
            4 => Code::DeadlineExceeded,
            5 => Code::NotFound,
            6 => Code::AlreadyExists,
            7 => Code::PermissionDenied,
            8 => Code::ResourceExhausted,
            9 => Code::FailedPrecondition,
            10 => Code::Aborted,
            11 => Code::OutOfRange,
            12 => Code::Unimplemented,
            13 => Code::Internal,
            14 => Code::Unavailable,
            15 => Code::DataLoss,
            16 => Code::Unauthenticated,
            _ => Code::Other(value),
        }
    }
}

impl Code {
    /// Returns the code number as a `u16`
    #[inline]
    pub fn as_u16(&self) -> u16 {
        match self {
            Code::Ok => 0,
            Code::Cancelled => 1,
            Code::Unknown => 2,
            Code::InvalidArgument => 3,
            Code::DeadlineExceeded => 4,
            Code::NotFound => 5,
            Code::AlreadyExists => 6,
            Code::PermissionDenied => 7,
            Code::ResourceExhausted => 8,
            Code::FailedPrecondition => 9,
            Code::Aborted => 10,
            Code::OutOfRange => 11,
            Code::Unimplemented => 12,
            Code::Internal => 13,
            Code::Unavailable => 14,
            Code::DataLoss => 15,
            Code::Unauthenticated => 16,
            Code::Other(value) => *value,
        }
    }

    fn header_value(&self) -> HeaderValue {
        match self {
            Code::Ok => HeaderValue::from_static("0"),
            Code::Cancelled => HeaderValue::from_static("1"),
            Code::Unknown => HeaderValue::from_static("2"),
            Code::InvalidArgument => HeaderValue::from_static("3"),
            Code::DeadlineExceeded => HeaderValue::from_static("4"),
            Code::NotFound => HeaderValue::from_static("5"),
            Code::AlreadyExists => HeaderValue::from_static("6"),
            Code::PermissionDenied => HeaderValue::from_static("7"),
            Code::ResourceExhausted => HeaderValue::from_static("8"),
            Code::FailedPrecondition => HeaderValue::from_static("9"),
            Code::Aborted => HeaderValue::from_static("10"),
            Code::OutOfRange => HeaderValue::from_static("11"),
            Code::Unimplemented => HeaderValue::from_static("12"),
            Code::Internal => HeaderValue::from_static("13"),
            Code::Unavailable => HeaderValue::from_static("14"),
            Code::DataLoss => HeaderValue::from_static("15"),
            Code::Unauthenticated => HeaderValue::from_static("16"),
            Code::Other(code) => {
                let mut buffer = itoa::Buffer::new();
                buffer.format(*code).parse().unwrap()
            }
        }
    }
}

/// A GRPC status describing the result of an RPC call.
#[derive(Debug, Clone)]
pub struct Status {
    code: Code,
    message: Option<String>,
    metadata: Metadata,
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(message) = &self.message {
            write!(
                f,
                "grpc status: code={}, message={}",
                self.code.as_u16(),
                message
            )
        } else {
            write!(f, "grpc status: code={}", self.code.as_u16())
        }
    }
}

impl std::error::Error for Status {}

impl Status {
    /// Create a `Status` with code
    #[inline]
    pub fn new(code: Code) -> Self {
        Self {
            code,
            message: None,
            metadata: Default::default(),
        }
    }

    /// Create a `Status` from an error that implements [`std::error::Error`].
    #[inline]
    pub fn from_std_error<T: std::error::Error>(err: T) -> Self {
        Status::new(Code::Internal).with_message(err)
    }

    /// Returns the code of this status
    #[inline]
    pub fn code(&self) -> Code {
        self.code
    }

    /// Returns `true` if the code is `Ok(0)`
    #[inline]
    pub fn is_ok(&self) -> bool {
        self.code == Code::Ok
    }

    /// Returns the reference to the error message.
    #[inline]
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Attach a error message to this status.
    #[inline]
    pub fn with_message(self, message: impl Display) -> Self {
        Self {
            message: Some(message.to_string()),
            ..self
        }
    }

    /// Attach a meta data to this status.
    #[inline]
    pub fn with_metadata(self, metadata: Metadata) -> Self {
        Self { metadata, ..self }
    }

    /// Returns a reference to the metadata.
    #[inline]
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Returns a mutable reference to the metadata.
    #[inline]
    pub fn metadata_mut(&mut self) -> &mut Metadata {
        &mut self.metadata
    }

    pub(crate) fn to_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

        headers.insert(GRPC_STATUS_HEADER_CODE, self.code.header_value());

        if let Some(message) = self
            .message
            .as_ref()
            .map(|message| percent_encode(message.as_bytes(), ENCODING_SET).to_string())
            .and_then(|encoded_message| HeaderValue::from_maybe_shared(encoded_message).ok())
        {
            headers.insert(GRPC_STATUS_MESSAGE_HEADER, message);
        }

        headers
    }

    pub(crate) fn from_headers(headers: &HeaderMap) -> Result<Option<Status>, Status> {
        if let Some(value) = headers.get(GRPC_STATUS_HEADER_CODE) {
            let code: Code = value
                .to_str()
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
                .ok_or_else(|| Status::new(Code::Internal).with_message("invalid grpc-status"))?
                .into();
            let mut status = Status::new(code);
            if let Some(message) = headers
                .get(GRPC_STATUS_MESSAGE_HEADER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| percent_decode_str(value).decode_utf8().ok())
            {
                status = status.with_message(message);
            }
            Ok(Some(status))
        } else {
            Ok(None)
        }
    }
}
