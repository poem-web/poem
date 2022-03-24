use std::{borrow::Cow, pin::Pin};

use poem::{
    http::{HeaderMap, StatusCode},
    Request,
};
use tokio::{
    io::AsyncRead,
    sync::mpsc::{channel, Receiver, Sender},
};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

#[derive(Debug)]
pub(crate) enum ResponseMsg {
    StatusCode(StatusCode),
    HeaderMap(HeaderMap),
    Body(Vec<u8>),
}

pub(crate) struct ExecState {
    pub(crate) wasi: WasiCtx,
    pub(crate) request: String,
    pub(crate) request_body: Pin<Box<dyn AsyncRead + Send + Sync>>,
    pub(crate) response_sender: Sender<ResponseMsg>,
}

impl ExecState {
    pub(crate) fn new(mut request: Request) -> (Self, Receiver<ResponseMsg>) {
        let wasi = WasiCtxBuilder::new().inherit_stdout().build();
        let (tx, rx) = channel(8);
        let request_body = Box::pin(request.take_body().into_async_read());
        (
            Self {
                wasi,
                request: build_request_string(&request),
                request_body,
                response_sender: tx,
            },
            rx,
        )
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
