pub const ERRNO_OK: i32 = 0;
pub const ERRNO_WOULD_BLOCK: i32 = -1;

extern "C" {
    pub fn read_request_body(buf: u32, buf_len: u32, bytes_read: u32) -> i32;

    pub fn set_response_status(code: u32);

    pub fn set_response_headers(buf: u32, buf_len: u32);

    pub fn write_response_body(buf: u32, buf_len: u32, bytes_written: u32) -> i32;

    pub fn poll(subscriptions: u32, num_subscriptions: u32, events: u32) -> u32;
}

pub const SUBSCRIPTION_TYPE_TIMEOUT: u8 = 1;
pub const SUBSCRIPTION_TYPE_REQUEST_READ: u8 = 2;
pub const SUBSCRIPTION_TYPE_RESPONSE_WRITE: u8 = 3;

#[repr(C)]
pub struct Subscription {
    ty: u8,
    userdata: u32,
    detail: SubscriptionDetail,
}
