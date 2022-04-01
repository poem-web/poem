use std::{
    future::Future,
    io::Cursor,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Bytes, BytesMut};
use hyper::body::HttpBody;
use poem_wasm::ffi::{RESPONSE_BODY_BYTES, RESPONSE_BODY_EMPTY, RESPONSE_BODY_STREAM};
use tokio::io::AsyncReadExt;

use crate::{runtime::wasi::request_reader::RequestReader, Body, Endpoint, Request};

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

        let mut resp = ep.get_response(request).await;
        let resp_status = resp.status();
        let resp_headers = std::mem::take(resp.headers_mut());
        let res = CheckResponseBody::new(resp.into_body()).await;

        match res {
            CheckResponseBodyResult::Empty => {
                poem_wasm::send_response(resp_status, &resp_headers, RESPONSE_BODY_EMPTY);
            }
            CheckResponseBodyResult::Bytes(data) => {
                poem_wasm::send_response(resp_status, &resp_headers, RESPONSE_BODY_BYTES);
                let _ = poem_wasm::write_response_body(&data);
            }
            CheckResponseBodyResult::Stream(mut body) => {
                poem_wasm::send_response(resp_status, &resp_headers, RESPONSE_BODY_STREAM);

                while let Some(Ok(data)) = body.data().await {
                    if poem_wasm::write_response_body(&data).is_err() {
                        break;
                    }
                }
            }
        }
    });
}

enum CheckResponseBodyResult {
    Empty,
    Bytes(Bytes),
    Stream(Body),
}

struct CheckResponseBody {
    body: hyper::Body,
}

impl CheckResponseBody {
    fn new(body: Body) -> Self {
        Self { body: body.into() }
    }
}

impl Future for CheckResponseBody {
    type Output = CheckResponseBodyResult;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;

        match Pin::new(&mut this.body).poll_data(cx) {
            Poll::Ready(Some(Ok(data))) => {
                let mut bytes = BytesMut::from(&*data);

                loop {
                    match Pin::new(&mut this.body).poll_data(cx) {
                        Poll::Ready(Some(Ok(data))) => {
                            bytes.extend_from_slice(&data);
                        }
                        Poll::Ready(None) | Poll::Ready(Some(Err(_))) => {
                            break Poll::Ready(CheckResponseBodyResult::Bytes(bytes.freeze()));
                        }
                        Poll::Pending => {
                            break Poll::Ready(CheckResponseBodyResult::Stream(
                                Body::from_async_read(Cursor::new(data).chain(
                                    Body::from(std::mem::take(&mut this.body)).into_async_read(),
                                )),
                            ));
                        }
                    }
                }
            }
            Poll::Ready(None) | Poll::Ready(Some(Err(_))) => {
                Poll::Ready(CheckResponseBodyResult::Empty)
            }
            Poll::Pending => Poll::Ready(CheckResponseBodyResult::Stream(
                std::mem::take(&mut this.body).into(),
            )),
        }
    }
}
