use poem::{error::ResponseError, http::StatusCode};
use wasmtime::Trap;

#[derive(Debug, thiserror::Error)]
pub enum WasmHandlerError {
    #[error("memory not found")]
    MemoryNotFound,
    #[error("memory access error")]
    MemoryAccess,
    #[error("invalid status code")]
    InvalidStatusCode,
    #[error("invalid header name")]
    InvalidHeaderName,
    #[error("invalid header value")]
    InvalidHeaderValue,
    #[error("invalid response")]
    InvalidResponse,
    #[error("incomplete response")]
    IncompleteResponse,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<WasmHandlerError> for Trap {
    fn from(err: WasmHandlerError) -> Self {
        let err: Box<dyn std::error::Error + Send + Sync> = Box::new(err);
        Trap::from(err)
    }
}

impl ResponseError for WasmHandlerError {
    fn status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}
