use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
};

use poem::{
    http::{HeaderMap, StatusCode},
    Request, Result,
};
use tokio::sync::mpsc;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

#[derive(Debug)]
pub(crate) enum ResponseMsg {
    StatusCode(StatusCode),
    HeaderMap(HeaderMap),
    Body(Vec<u8>),
}

pub struct WasmEndpointState<State = ()> {
    pub(crate) wasi: WasiCtx,
    pub(crate) user_state: State,
    pub(crate) request: String,
    pub(crate) request_body: Vec<u8>,
    pub(crate) response_sender: mpsc::UnboundedSender<ResponseMsg>,
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
    pub(crate) async fn new(
        mut request: Request,
        user_state: State,
    ) -> Result<(Self, mpsc::UnboundedReceiver<ResponseMsg>)> {
        let wasi = WasiCtxBuilder::new().inherit_stdout().build();
        let (tx, rx) = mpsc::unbounded_channel();
        let request_body = request.take_body().into_vec().await?;
        Ok((
            Self {
                wasi,
                user_state,
                request: build_request_string(&request),
                request_body,
                response_sender: tx,
            },
            rx,
        ))
    }
}

fn build_request_string(request: &Request) -> String {
    let mut iter = std::iter::once(Cow::Borrowed(request.method().as_str()))
        .chain(std::iter::once(Cow::Owned(request.uri().to_string())))
        .chain(
            request
                .headers()
                .iter()
                .filter_map(|(name, value)| value.to_str().map(|value| (name.as_str(), value)).ok())
                .map(|(name, value)| {
                    std::iter::once(Cow::Borrowed(name))
                        .chain(std::iter::once(Cow::Borrowed(value)))
                })
                .flatten(),
        );
    let mut s = String::new();

    if let Some(value) = iter.next() {
        s.push_str(&value);
    } else {
        return s;
    }

    for value in iter {
        s.push_str("\n");
        s.push_str(&value);
    }

    s
}
