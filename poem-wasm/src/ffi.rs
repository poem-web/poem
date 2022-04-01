pub const ERRNO_OK: i32 = 0;
pub const ERRNO_WOULD_BLOCK: i32 = -1;
pub const ERRNO_UNKNOWN: i32 = -500;

pub const RESPONSE_BODY_EMPTY: u32 = 0;
pub const RESPONSE_BODY_BYTES: u32 = 1;
pub const RESPONSE_BODY_STREAM: u32 = 2;

#[cfg(target_os = "wasi")]
#[link(wasm_import_module = "poem")]
extern "C" {
    pub fn read_request(buf: u32, buf_len: u32, request_len: u32);

    pub fn read_request_body(buf: u32, buf_len: u32, bytes_read: u32) -> i32;

    pub fn send_response(code: u32, headers_buf: u32, headers_buf_len: u32, body_type: u32);

    pub fn write_response_body(buf: u32, buf_len: u32) -> i32;

    pub fn read_upgraded(buf: u32, buf_len: u32, bytes_read: u32) -> i32;

    pub fn write_upgraded(buf: u32, buf_len: u32) -> i32;

    pub fn poll(subscriptions: u32, num_subscriptions: u32, event: u32);
}

pub const SUBSCRIPTION_TYPE_TIMEOUT: u8 = 1;
pub const SUBSCRIPTION_TYPE_REQUEST_READ: u8 = 2;
pub const SUBSCRIPTION_TYPE_UPGRADED_READ: u8 = 3;

#[repr(C)]
pub struct RawSubscription {
    pub ty: u8,
    pub userdata: u64,
    pub deadline: u64,
}

#[repr(C)]
pub struct RawEvent {
    pub ty: u8,
    pub userdata: u64,
    pub errno: i32,
}
