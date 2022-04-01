use std::ops::{Deref, DerefMut};

use bytes::BytesMut;
use poem::{
    http::{HeaderMap, StatusCode},
    Request,
};
use tokio::io::DuplexStream;
use tokio::{io::AsyncRead, sync::mpsc};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

pub(crate) struct UpgradedState {
    pub(crate) reader_buf: BytesMut,
    pub(crate) reader_eof: bool,
    pub(crate) reader: DuplexStream,
    pub(crate) sender: mpsc::UnboundedSender<Vec<u8>>,
}

pub struct WasmEndpointState<State = ()> {
    pub(crate) wasi: WasiCtx,
    pub(crate) user_state: State,
    pub(crate) request: String,
    pub(crate) response_sender: mpsc::UnboundedSender<(StatusCode, HeaderMap, u32)>,
    pub(crate) request_body_buf: BytesMut,
    pub(crate) request_body_eof: bool,
    pub(crate) request_body_reader: Box<dyn AsyncRead + Send + Unpin>,
    pub(crate) response_body_sender: mpsc::UnboundedSender<Vec<u8>>,
    pub(crate) upgraded: Option<UpgradedState>,
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
    pub(crate) fn new(
        mut request: Request,
        response_sender: mpsc::UnboundedSender<(StatusCode, HeaderMap, u32)>,
        response_body_sender: mpsc::UnboundedSender<Vec<u8>>,
        upgraded: Option<(DuplexStream, mpsc::UnboundedSender<Vec<u8>>)>,
        user_state: State,
    ) -> Self {
        let wasi = WasiCtxBuilder::new().inherit_stdout().build();
        let request_body_reader = Box::new(request.take_body().into_async_read());

        Self {
            wasi,
            user_state,
            request: poem_wasm::encode_request(request.method(), request.uri(), request.headers()),
            response_sender,
            request_body_buf: BytesMut::new(),
            request_body_eof: false,
            request_body_reader,
            response_body_sender,
            upgraded: upgraded.map(|(reader, sender)| UpgradedState {
                reader_buf: BytesMut::new(),
                reader_eof: false,
                reader,
                sender,
            }),
        }
    }
}
