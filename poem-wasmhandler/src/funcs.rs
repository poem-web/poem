use std::str::FromStr;

use poem::{
    http::{header::HeaderName, HeaderMap, HeaderValue, StatusCode},
    Result,
};
use wasmtime::{AsContextMut, Caller, Engine, Extern, Linker, Memory, Trap};

use crate::{
    state::{ExecState, ResponseMsg},
    WasmHandlerError,
};

pub(crate) fn create_linker(engine: &Engine) -> Result<Linker<ExecState>> {
    let mut linker = Linker::new(&engine);

    linker.func_wrap("env", "request_get", request_get)?;
    linker.func_wrap("env", "request_get_body", request_get_body)?;
    linker.func_wrap("env", "response_status", response_status)?;
    linker.func_wrap("env", "response_header_map", response_header_map)?;
    linker.func_wrap("env", "response_body", response_body)?;

    Ok(linker)
}

fn get_memory<T>(caller: &mut Caller<'_, T>) -> Result<Memory, WasmHandlerError> {
    if let Some(Extern::Memory(memory)) = caller.get_export("memory") {
        Ok(memory)
    } else {
        Err(WasmHandlerError::MemoryNotFound)
    }
}

#[inline]
fn get_memory_slice_mut(
    memory: &mut [u8],
    offset: u32,
    len: u32,
) -> Result<&mut [u8], WasmHandlerError> {
    memory
        .get_mut(offset as usize..(offset + len) as usize)
        .ok_or_else(|| WasmHandlerError::MemoryAccess)
}

#[inline]
fn set_ret_len(memory: &mut [u8], buf: u32, ret_len: u32) -> Result<(), WasmHandlerError> {
    get_memory_slice_mut(memory, buf, 4)?.copy_from_slice(&ret_len.to_le_bytes());
    Ok(())
}

fn request_get(
    mut caller: Caller<'_, ExecState>,
    buf: u32,
    buf_len: u32,
    ret_len: u32,
) -> Result<i32, Trap> {
    let memory = get_memory(&mut caller)?;
    let (memory, state) = memory.data_and_store_mut(caller.as_context_mut());

    if buf_len < state.request.len() as u32 {
        set_ret_len(memory, ret_len, state.request.len() as u32)?;
        return Ok(-1);
    }

    get_memory_slice_mut(memory, buf, state.request.len() as u32)?
        .copy_from_slice(state.request.as_bytes());
    set_ret_len(memory, ret_len, state.request.len() as u32)?;
    Ok(0)
}

fn request_get_body(
    mut caller: Caller<'_, ExecState>,
    buf: u32,
    buf_len: u32,
    ret_len: u32,
) -> Result<i32, Trap> {
    let memory = get_memory(&mut caller)?;
    let (memory, state) = memory.data_and_store_mut(caller.as_context_mut());

    if buf_len < state.request_body.len() as u32 {
        set_ret_len(memory, ret_len, state.request_body.len() as u32)?;
        return Ok(-1);
    }

    get_memory_slice_mut(memory, buf, state.request_body.len() as u32)?
        .copy_from_slice(&state.request_body);
    set_ret_len(memory, ret_len, state.request_body.len() as u32)?;
    Ok(0)
}

pub(crate) fn response_status<'a>(caller: Caller<'a, ExecState>, status: u32) -> Result<(), Trap> {
    if status > u16::MAX as u32 {
        return Err(WasmHandlerError::InvalidStatusCode.into());
    }

    let status =
        StatusCode::from_u16(status as u16).map_err(|_| WasmHandlerError::InvalidStatusCode)?;
    let _ = caller
        .data()
        .response_sender
        .send(ResponseMsg::StatusCode(status));
    Ok(())
}

pub(crate) fn response_header_map<'a>(
    mut caller: Caller<'a, ExecState>,
    data: u32,
    data_len: u32,
) -> Result<(), Trap> {
    let memory = get_memory(&mut caller)?;
    let (memory, _) = memory.data_and_store_mut(caller.as_context_mut());

    let data = get_memory_slice_mut(memory, data, data_len)?;
    let data = std::str::from_utf8(data).expect("valid header map");
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

    let _ = caller
        .data()
        .response_sender
        .send(ResponseMsg::HeaderMap(headers));
    Ok(())
}

pub(crate) fn response_body<'a>(
    mut caller: Caller<'a, ExecState>,
    data: u32,
    data_len: u32,
) -> Result<i32, Trap> {
    let memory = get_memory(&mut caller)?;
    let (memory, state) = memory.data_and_store_mut(caller.as_context_mut());

    let data = get_memory_slice_mut(memory, data, data_len)?;
    let res = state.response_sender.send(ResponseMsg::Body(data.to_vec()));
    Ok(if res.is_ok() { 0 } else { -1 })
}
