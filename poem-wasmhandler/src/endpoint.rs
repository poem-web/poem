use poem::{
    error::InternalServerError, http::StatusCode, Body, Endpoint, Error, Request, Response, Result,
};
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt};
use wasmtime::{Config, Engine, Linker, Module, Store};

use crate::{
    funcs,
    state::{ExecState, ResponseMsg},
    WasmHandlerError,
};

pub struct WasmEndpoint {
    engine: Engine,
    module: Module,
    linker: Linker<ExecState>,
}

impl WasmEndpoint {
    pub fn new(bytes: impl AsRef<[u8]>) -> Result<Self> {
        let engine = Engine::new(&Config::new().async_support(true))?;
        let module = Module::new(&engine, bytes)?;
        let mut linker = funcs::create_linker(&engine)?;
        wasmtime_wasi::add_to_linker(&mut linker, |s| &mut s.wasi)?;

        Ok(Self {
            engine,
            module,
            linker,
        })
    }
}

#[poem::async_trait]
impl Endpoint for WasmEndpoint {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        // create wasm instance
        let (state, mut body_receiver) = ExecState::new(req).await?;
        let mut store = Store::new(&self.engine, state);

        tracing::debug!("instantiate WASM module");
        let instance = self
            .linker
            .instantiate_async(&mut store, &self.module)
            .await?;

        // invoke main
        tracing::debug!("execute start");
        let (tx_exec, mut rx_exec) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let start_func = match instance.get_typed_func::<(), (), _>(&mut store, "start") {
                Ok(start_func) => start_func,
                Err(err) => {
                    let _ = tx_exec.send(Err(Error::from(err)));
                    return;
                }
            };

            let _ = tx_exec.send(
                start_func
                    .call_async(&mut store, ())
                    .await
                    .map_err(InternalServerError),
            );
        });

        // create response
        let mut status = StatusCode::OK;

        loop {
            tokio::select! {
                res = rx_exec.recv() => {
                    if let Some(Err(err)) = res {
                        return Err(err);
                    }
                }
                item = body_receiver.recv() => {
                    match item {
                        Some(ResponseMsg::StatusCode(value)) => status = value,
                        Some(ResponseMsg::HeaderMap(value)) => {
                            let mut resp = Response::default();
                            resp.set_status(status);
                            *resp.headers_mut() = value;
                            resp.set_body(Body::from_bytes_stream(wrap_body_stream(body_receiver)));
                            return Ok(resp);
                        },
                        Some(ResponseMsg::Body(_)) => return Err(WasmHandlerError::InvalidResponse.into()),
                        None => return Err(WasmHandlerError::IncompleteResponse.into()),
                    }
                }
            }
        }
    }
}

fn wrap_body_stream(
    receiver: mpsc::UnboundedReceiver<ResponseMsg>,
) -> impl Stream<Item = Result<Vec<u8>, std::io::Error>> {
    tokio_stream::wrappers::UnboundedReceiverStream::new(receiver)
        .filter_map(|msg| match msg {
            ResponseMsg::Body(data) => Some(data),
            _ => None,
        })
        .map(Ok)
}
