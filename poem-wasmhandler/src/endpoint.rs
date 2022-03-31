use poem::{Body, Endpoint, Request, Response, Result};
use tokio::sync::mpsc;
use wasmtime::{Config, Engine, IntoFunc, Linker, Module, Store};

use crate::{funcs, state::WasmEndpointState, WasmHandlerError};

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
        let engine = Engine::new(&Config::new().async_support(true)).unwrap();
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
        wasmtime_wasi::add_to_linker(&mut self.linker, |state| &mut state.wasi)?;

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
        let (mut response_receiver, response_body_reader) = {
            let user_state = self.user_state.clone();
            let (response_sender, response_receiver) = mpsc::unbounded_channel();
            let (response_body_writer, response_body_reader) = tokio::io::duplex(8192);
            let state =
                WasmEndpointState::new(req, response_sender, response_body_writer, user_state);
            let mut store = Store::new(&self.engine, state);
            let linker = self.linker.clone();
            let module = self.module.clone();

            // invoke main
            tokio::spawn(async move {
                let instance = match linker.instantiate_async(&mut store, &module).await {
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
                let _ = start_func.call_async(&mut store, ()).await;
            });

            (response_receiver, response_body_reader)
        };

        // receive response
        match response_receiver.recv().await {
            Some((status, headers)) => {
                let mut resp = Response::default();
                resp.set_status(status);
                *resp.headers_mut() = headers;
                resp.set_body(Body::from_async_read(response_body_reader));
                Ok(resp)
            }
            None => Err(WasmHandlerError::IncompleteResponse.into()),
        }
    }
}
