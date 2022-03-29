use poem::{http::StatusCode, Body, Endpoint, Request, Response, Result};
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt};
use wasmtime::{Engine, IntoFunc, Linker, Module, Store};

use crate::{
    funcs,
    state::{ResponseMsg, WasmEndpointState},
    WasmHandlerError,
};

pub struct WasmEndpointBuilder<State>
where
    State: Send + Sync + Clone + 'static,
{
    engine: Engine,
    linker: Linker<WasmEndpointState<State>>,
    module: Vec<u8>,
    user_state: State,
}

impl WasmEndpointBuilder<()> {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        let engine = Engine::default();
        let linker = Linker::new(&engine);

        Self {
            engine,
            linker,
            module: bytes.into(),
            user_state: (),
        }
    }
}

impl<State> WasmEndpointBuilder<State>
where
    State: Send + Sync + Clone + 'static,
{
    pub fn with_state(self, user_state: State) -> WasmEndpointBuilder<State> {
        Self {
            user_state,
            linker: Linker::new(&self.engine),
            ..self
        }
    }

    pub fn udf<Params, Args>(
        mut self,
        module: &str,
        name: &str,
        func: impl IntoFunc<WasmEndpointState<State>, Params, Args>,
    ) -> Self {
        self.linker.func_wrap(module, name, func).unwrap();
        self
    }

    pub fn build(mut self) -> Result<WasmEndpoint<State>> {
        let module = Module::new(&self.engine, self.module)?;
        funcs::add_to_linker(&mut self.linker).unwrap();
        wasmtime_wasi::add_to_linker(&mut self.linker, |state| &mut state.wasi).unwrap();

        Ok(WasmEndpoint {
            engine: self.engine,
            module,
            linker: self.linker,
            user_state: self.user_state,
        })
    }
}

pub struct WasmEndpoint<State> {
    engine: Engine,
    module: Module,
    linker: Linker<WasmEndpointState<State>>,
    user_state: State,
}

#[poem::async_trait]
impl<State> Endpoint for WasmEndpoint<State>
where
    State: Send + Sync + Clone + 'static,
{
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        // create wasm instance
        let mut response_receiver = {
            let user_state = self.user_state.clone();
            let (state, response_receiver) = WasmEndpointState::new(req, user_state).await?;
            let mut store = Store::new(&self.engine, state);
            let linker = self.linker.clone();
            let module = self.module.clone();

            // invoke main
            tokio::task::spawn_blocking(move || {
                let instance = match linker.instantiate(&mut store, &module) {
                    Ok(instance) => instance,
                    Err(err) => {
                        tracing::error!(error = %err, "wasm instantiate error");
                        return;
                    }
                };
                let start_func = match instance.get_typed_func::<(), (), _>(&mut store, "start") {
                    Ok(start_func) => start_func,
                    Err(err) => {
                        tracing::error!(error = %err, "wasm error");
                        return;
                    }
                };
                let _ = start_func.call(&mut store, ());
            });

            response_receiver
        };

        // create response
        let mut status = StatusCode::SERVICE_UNAVAILABLE;

        while let Some(msg) = response_receiver.recv().await {
            match msg {
                ResponseMsg::StatusCode(value) => status = value,
                ResponseMsg::HeaderMap(value) => {
                    let mut resp = Response::default();
                    resp.set_status(status);
                    *resp.headers_mut() = value;
                    resp.set_body(Body::from_bytes_stream(wrap_body_stream(response_receiver)));
                    return Ok(resp);
                }
                ResponseMsg::Body(_) => return Err(WasmHandlerError::InvalidResponse.into()),
            }
        }

        Err(WasmHandlerError::IncompleteResponse.into())
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
