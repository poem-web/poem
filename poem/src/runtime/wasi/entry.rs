use crate::{runtime::wasi::request_reader::RequestReader, Body, Endpoint, Request};
use tokio::io::AsyncReadExt;

pub fn run<E>(ep: E)
where
    E: Endpoint + 'static,
{
    crate::runtime::wasi::task::block_on(async move {
        let (method, uri, headers) = poem_wasm::get_request();
        let request = {
            let mut request = Request::default();
            request.set_method(method);
            *request.uri_mut() = uri;
            *request.headers_mut() = headers;
            request.set_body(Body::from_async_read(RequestReader));
            request
        };

        let resp = ep.get_response(request).await;
        poem_wasm::send_response(resp.status(), resp.headers());

        let mut reader = resp.into_body().into_async_read();
        loop {
            let mut data = [0; 4096];
            match reader.read(&mut data).await {
                Ok(0) | Err(_) => break,
                Ok(sz) => {
                    if poem_wasm::write_response_body(&data[..sz]).is_err() {
                        break;
                    }
                }
            }
        }
    });
}
