pub mod ffi;

use crate::ffi::{RawEvent, RawSubscription};
use http::header::HeaderName;
use http::{HeaderMap, HeaderValue, Method, StatusCode, Uri};
use std::borrow::Cow;
use std::io::{Error, ErrorKind, Result};
use std::mem::MaybeUninit;
use std::str::FromStr;

pub struct Subscription(RawSubscription);

impl Subscription {
    #[inline]
    pub fn timeout(deadline: u64) -> Self {
        Self(RawSubscription {
            ty: ffi::SUBSCRIPTION_TYPE_TIMEOUT,
            userdata: 0,
            deadline,
        })
    }

    #[inline]
    pub fn read_request_body() -> Self {
        Self(RawSubscription {
            ty: ffi::SUBSCRIPTION_TYPE_REQUEST_READ,
            userdata: 0,
            deadline: 0,
        })
    }

    #[inline]
    pub fn write_response_body() -> Self {
        Self(RawSubscription {
            ty: ffi::SUBSCRIPTION_TYPE_RESPONSE_WRITE,
            userdata: 0,
            deadline: 0,
        })
    }

    #[inline]
    pub fn userdata(self, data: u64) -> Self {
        Self(RawSubscription {
            userdata: data,
            ..self.0
        })
    }

    #[inline]
    pub fn get_userdata(&self) -> u64 {
        self.0.userdata
    }
}

pub struct Event(RawEvent);

impl Event {
    #[inline]
    pub fn userdata(&self) -> u64 {
        self.0.userdata
    }

    #[inline]
    pub fn errno(&self) -> i32 {
        self.0.errno
    }
}

#[cfg(target_os = "wasi")]
pub fn read_request_body(data: &mut [u8]) -> Result<usize> {
    let mut bytes_read = 0u32;

    unsafe {
        match ffi::read_request_body(
            data.as_mut_ptr() as u32,
            data.len() as u32,
            &mut bytes_read as *mut _ as u32,
        ) {
            ffi::ERRNO_OK => Ok(bytes_read as usize),
            ffi::ERRNO_WOULD_BLOCK => Err(Error::new(ErrorKind::WouldBlock, "would block")),
            _ => Err(Error::new(ErrorKind::Other, "other")),
        }
    }
}

#[cfg(target_os = "wasi")]
pub fn write_response_body(data: &[u8]) -> Result<usize> {
    let mut bytes_written = 0u32;

    unsafe {
        match ffi::write_response_body(
            data.as_ptr() as u32,
            data.len() as u32,
            &mut bytes_written as *mut _ as u32,
        ) {
            ffi::ERRNO_OK => Ok(bytes_written as usize),
            ffi::ERRNO_WOULD_BLOCK => Err(Error::new(ErrorKind::WouldBlock, "would block")),
            _ => Err(Error::new(ErrorKind::Other, "other")),
        }
    }
}

#[cfg(target_os = "wasi")]
pub fn send_response(status: StatusCode, headers: &HeaderMap) {
    let s = encode_headers(headers);
    unsafe { ffi::send_response(status.as_u16() as u32, s.as_ptr() as u32, s.len() as u32) }
}

pub fn decode_headers(data: &str) -> HeaderMap {
    let mut iter = data.split('\n');
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

    headers
}

fn encode_headers(headers: &HeaderMap) -> String {
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

#[cfg(target_os = "wasi")]
pub fn poll(subscriptions: &[Subscription]) -> Event {
    unsafe {
        let mut event: MaybeUninit<RawEvent> = MaybeUninit::uninit();
        ffi::poll(
            subscriptions.as_ptr() as u32,
            subscriptions.len() as u32,
            event.as_mut_ptr() as u32,
        );
        Event(event.assume_init())
    }
}

pub fn encode_request(method: &Method, uri: &Uri, headers: &HeaderMap) -> String {
    let mut iter = std::iter::once(Cow::Borrowed(method.as_str()))
        .chain(std::iter::once(Cow::Owned(uri.to_string())))
        .chain(
            headers
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

#[cfg(target_os = "wasi")]
pub fn get_request() -> (Method, Uri, HeaderMap) {
    unsafe {
        let mut request_len = 0u32;
        let mut data = Vec::new();

        ffi::read_request(0, 0, &mut request_len as *mut _ as u32);
        data.reserve(request_len as usize);
        ffi::read_request(
            data.as_mut_ptr() as u32,
            data.capacity() as u32,
            &mut request_len as *mut _ as u32,
        );
        data.set_len(request_len as usize);

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

        (method, uri, headers)
    }
}
