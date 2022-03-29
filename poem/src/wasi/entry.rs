use std::str::FromStr;

use tokio::io::AsyncReadExt;

use crate::{
    http::{
        header::{HeaderName, HeaderValue},
        HeaderMap, Method, Uri,
    },
    wasi::task::block_on as wasi_block_on,
    Endpoint, Request,
};

mod ffi {
    extern "C" {
        pub(super) fn request_get(buf: u32, buf_len: u32, ret_buf_len: u32) -> i32;

        pub(super) fn request_get_body(buf: u32, buf_len: u32, ret_buf_len: u32) -> i32;

        pub(super) fn response_status(status: u32);

        pub(super) fn response_header_map(data: u32, data_len: u32);

        pub(super) fn response_body(data: u32, data_len: u32) -> i32;
    }
}

pub fn run<E>(ep: E)
where
    E: Endpoint + 'static,
{
    wasi_block_on(async move {
        unsafe {
            let request = create_request();

            let resp = ep.get_response(request).await;
            let headers_str = encode_header_map_string(resp.headers());

            ffi::response_status(resp.status().as_u16() as u32);
            ffi::response_header_map(headers_str.as_ptr() as u32, headers_str.len() as u32);

            let mut reader = resp.into_body().into_async_read();
            loop {
                let mut buf = [0; 4096];
                match reader.read(&mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if ffi::response_body(buf.as_ptr() as u32, n as u32) != 0 {
                            break;
                        }
                    }
                }
            }
        }
    });
}

unsafe fn get_request_data() -> Vec<u8> {
    let mut data_len = 0u32;

    debug_assert_eq!(ffi::request_get(0, 0, &mut data_len as *mut u32 as u32), -1);
    let mut data = vec![0u8; data_len as usize];

    debug_assert_eq!(
        ffi::request_get(
            data.as_mut_ptr() as u32,
            data.len() as u32,
            &mut data_len as *mut u32 as u32,
        ),
        0
    );

    data
}

unsafe fn get_request_body() -> Vec<u8> {
    let mut data_len = 0u32;

    ffi::request_get_body(0, 0, &mut data_len as *mut u32 as u32);
    let mut data = vec![0u8; data_len as usize];

    debug_assert_eq!(
        ffi::request_get_body(
            data.as_mut_ptr() as u32,
            data.len() as u32,
            &mut data_len as *mut u32 as u32,
        ),
        0
    );

    data
}

unsafe fn create_request() -> Request {
    let request_data = get_request_data();
    let mut iter = std::str::from_utf8_unchecked(&request_data).split('\n');
    let method_str = iter.next().unwrap();
    let method = method_str.parse::<Method>().unwrap();
    let uri_str = iter.next().unwrap();
    let uri = uri_str.parse::<Uri>().unwrap();

    let mut headers = HeaderMap::new();

    loop {
        let name = iter.next();
        let value = iter.next();

        if let Some((name, value)) = name.zip(value) {
            headers.append(
                HeaderName::from_str(name).expect("valid header name"),
                HeaderValue::from_str(value).expect("valid header value"),
            );
        } else {
            break;
        }
    }

    let mut request = Request::default();
    request.set_method(method);
    *request.uri_mut() = uri;
    *request.headers_mut() = headers;
    request.set_body(get_request_body());

    request
}

fn encode_header_map_string(headers: &HeaderMap) -> String {
    let mut iter = headers
        .iter()
        .filter_map(|(name, value)| value.to_str().map(|value| (name.as_str(), value)).ok())
        .map(|(name, value)| std::iter::once(name).chain(std::iter::once(value)))
        .flatten();
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
