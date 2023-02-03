use std::io::Result as IoResult;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use hyper::{body::HttpBody, HeaderMap};
use poem::Body;

use crate::{
    codec::{Decoder, Encoder},
    Code, Status, Streaming,
};

fn encode_data_frame<T: Encoder>(
    encoder: &mut T,
    buf: &mut BytesMut,
    message: T::Item,
) -> IoResult<Bytes> {
    buf.put_slice(&[0, 0, 0, 0, 0]);
    encoder.encode(message, buf)?;
    let msg_len = (buf.len() - 5) as u32;
    buf.as_mut()[1..5].copy_from_slice(&msg_len.to_be_bytes());
    Ok(buf.split().freeze())
}

#[derive(Default)]
struct DataFrameDecoder {
    buf: BytesMut,
}

impl DataFrameDecoder {
    fn put_slice(&mut self, data: impl AsRef<[u8]>) {
        self.buf.extend_from_slice(data.as_ref());
    }

    fn check_incomplete(&self) -> Result<(), Status> {
        if !self.buf.is_empty() {
            return Err(Status::new(Code::Internal).with_message("incomplete request"));
        }
        Ok(())
    }

    fn next(&mut self) -> Result<Option<Bytes>, Status> {
        if self.buf.len() < 5 {
            return Ok(None);
        }

        let compressed = match self.buf[0] {
            1 => true,
            0 => false,
            compressed => Err(Status::new(Code::Internal)
                .with_message(format!("unsupported compressed flag: {compressed}")))?,
        };

        let len = u32::from_be_bytes(self.buf[1..5].try_into().unwrap()) as usize;
        if self.buf.len() >= len + 5 {
            self.buf.advance(5);
            let data = self.buf.split_to(len).freeze();

            if compressed {
                let mut decoder = GzDecoder::new(&*data);
                let raw_data = BytesMut::new();
                let mut writer = raw_data.writer();
                std::io::copy(&mut decoder, &mut writer).map_err(Status::from_std_error)?;
                Ok(Some(writer.into_inner().freeze()))
            } else {
                Ok(Some(data))
            }
        } else {
            Ok(None)
        }
    }
}

pub(crate) fn create_decode_request_body<T: Decoder>(
    mut decoder: T,
    body: Body,
) -> Streaming<T::Item> {
    let mut body: hyper::Body = body.into();

    Streaming::new(async_stream::try_stream! {
        let mut frame_decoder = DataFrameDecoder::default();

        loop {
            match body.data().await.transpose().map_err(Status::from_std_error)? {
                Some(data) => {
                    frame_decoder.put_slice(data);
                    while let Some(data) = frame_decoder.next()? {
                        let message = decoder.decode(&data).map_err(Status::from_std_error)?;
                        yield message;
                    }
                }
                None => {
                    frame_decoder.check_incomplete()?;
                    break;
                }
            }
        }
    })
}

pub(crate) fn create_encode_response_body<T: Encoder>(
    mut encoder: T,
    mut stream: Streaming<T::Item>,
) -> Body {
    let (mut sender, body) = hyper::Body::channel();

    tokio::spawn(async move {
        let mut buf = BytesMut::new();

        while let Some(item) = stream.next().await {
            match item {
                Ok(message) => {
                    if let Ok(data) = encode_data_frame(&mut encoder, &mut buf, message) {
                        if sender.send_data(data).await.is_err() {
                            return;
                        }
                    }
                }
                Err(status) => {
                    let _ = sender.send_trailers(status.to_headers()).await;
                    return;
                }
            }
        }

        let _ = sender
            .send_trailers(Status::new(Code::Ok).to_headers())
            .await;
    });

    body.into()
}

pub(crate) fn create_encode_request_body<T: Encoder>(
    mut encoder: T,
    mut stream: Streaming<T::Item>,
) -> Body {
    let (mut sender, body) = hyper::Body::channel();

    tokio::spawn(async move {
        let mut buf = BytesMut::new();

        while let Some(Ok(message)) = stream.next().await {
            if let Ok(data) = encode_data_frame(&mut encoder, &mut buf, message) {
                if sender.send_data(data).await.is_err() {
                    return;
                }
            }
        }
    });

    body.into()
}

pub(crate) fn create_decode_response_body<T: Decoder>(
    mut decoder: T,
    headers: &HeaderMap,
    body: Body,
) -> Result<Streaming<T::Item>, Status> {
    // check is trailers-only
    if let Some(status) = Status::from_headers(headers)? {
        return if status.is_ok() {
            Ok(Streaming::new(futures_util::stream::empty()))
        } else {
            Err(status)
        };
    }

    let mut body: hyper::Body = body.into();

    Ok(Streaming::new(async_stream::try_stream! {
        let mut frame_decoder = DataFrameDecoder::default();

        loop {
            if let Some(data) = body.data().await.transpose().map_err(Status::from_std_error)? {
                frame_decoder.put_slice(data);
                while let Some(data) = frame_decoder.next()? {
                    let message = decoder.decode(&data).map_err(Status::from_std_error)?;
                    yield message;
                }
                continue;
            }

            frame_decoder.check_incomplete()?;

            match body.trailers().await.map_err(Status::from_std_error)? {
                Some(trailers) => {
                    let status = Status::from_headers(&trailers)?
                        .ok_or_else(|| Status::new(Code::Internal).with_message("missing grpc-status"))?;
                    if !status.is_ok() {
                        Err(status)?;
                    }
                }
                None => Err(Status::new(Code::Internal).with_message("missing trailers"))?,
            }

            break;
        }
    }))
}
