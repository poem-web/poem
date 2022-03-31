use crate::Request;

pub fn run<E>(ep: E)
where
    E: Endpoint + 'static,
{
    crate::runtime::wasi::task::block_on(async move {
        unsafe {
            let (method, uri, headers) = poem_wasm::get_request();
            let request = {
                let mut request = Request::default();
                request.set_method(method);
                *request.uri_mut() = uri;
                *request.headers_mut() = headers;
                request
            };

            let resp = ep.get_response(request).await;
            poem_wasm::send_response(resp.status(), resp.headers());
        }
    });
}
