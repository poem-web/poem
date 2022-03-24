use std::str::FromStr;

use poem::{
    http::{header::HeaderName, HeaderMap, HeaderValue, Method, Uri},
    Body, Endpoint, Request,
};
use tokio::io::AsyncReadExt;

use crate::{ffi, request_body::RequestBodyStream};

pub async fn run<E: Endpoint>(ep: E) {
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
}

unsafe fn create_request() -> Request {
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

    let mut iter = std::str::from_utf8_unchecked(&data).split('\n');
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
    request.set_body(Body::from_async_read(RequestBodyStream));

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
