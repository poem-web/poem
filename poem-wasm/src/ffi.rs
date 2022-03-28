extern "C" {
    pub(crate) fn request_get(buf: u32, buf_len: u32, ret_buf_len: u32) -> i32;

    pub(crate) fn request_get_body(buf: u32, buf_len: u32, ret_buf_len: u32) -> i32;

    pub(crate) fn response_status(status: u32);

    pub(crate) fn response_header_map(data: u32, data_len: u32);

    pub(crate) fn response_body(data: u32, data_len: u32) -> i32;
}
