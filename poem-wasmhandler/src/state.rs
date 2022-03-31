use bytes::BytesMut;
use std::ops::{Deref, DerefMut};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

use poem::{
    http::{HeaderMap, StatusCode},
    Request,
};

pub struct WasmEndpointState<State = ()> {
    pub(crate) wasi: WasiCtx,
    pub(crate) user_state: State,
    pub(crate) request: String,
    pub(crate) response_sender: mpsc::UnboundedSender<(StatusCode, HeaderMap)>,
    pub(crate) request_body_buf: BytesMut,
    pub(crate) request_body_eof: bool,
    pub(crate) response_body_buf: BytesMut,
    pub(crate) request_body_reader: Box<dyn AsyncRead + Send + Unpin>,
    pub(crate) response_body_writer: Box<dyn AsyncWrite + Send + Unpin>,
}

impl<State> Deref for WasmEndpointState<State> {
    type Target = State;

    fn deref(&self) -> &Self::Target {
        &self.user_state
    }
}

impl<State> DerefMut for WasmEndpointState<State> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.user_state
    }
}

impl<State> WasmEndpointState<State> {
    pub(crate) fn new<W>(
        mut request: Request,
        response_sender: mpsc::UnboundedSender<(StatusCode, HeaderMap)>,
        response_body_writer: W,
        user_state: State,
    ) -> Self
    where
        W: AsyncWrite + Send + Unpin + 'static,
    {
        let wasi = WasiCtxBuilder::new().inherit_stdout().build();
        let request_body_reader = Box::new(request.take_body().into_async_read());

        Self {
            wasi,
            user_state,
            request: poem_wasm::encode_request(request.method(), request.uri(), request.headers()),
            response_sender,
            request_body_buf: BytesMut::new(),
            request_body_eof: false,
            response_body_buf: BytesMut::new(),
            request_body_reader,
            response_body_writer: Box::new(response_body_writer),
        }
    }
}
