use std::{
    io::Result as IoResult,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use flate2::read::GzDecoder;
use futures_util::{stream::BoxStream, Stream, StreamExt};
use hyper::{body::HttpBody, HeaderMap};
use poem::Body;

use crate::{
    codec::{Decoder, Encoder},
    Code, Status,
};

/// Message stream
pub struct Streaming<T>(BoxStream<'static, Result<T, Status>>);

impl<T> Streaming<T> {
    /// Create a message stream
    #[inline]
    pub fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<T, Status>> + Send + 'static,
    {
        Self(stream.boxed())
    }
}

impl<T> Stream for Streaming<T> {
    type Item = Result<T, Status>;

    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.poll_next_unpin(cx)
    }
}

fn encode_data_frame<T: Encoder>(encoder: &mut T, message: T::Item) -> IoResult<Bytes> {
    let mut data = BytesMut::new();
    encoder.encode(message, &mut data)?;

    let mut frame_data = BytesMut::new();
    frame_data.put_u8(0);
    frame_data.put_u32(data.len() as u32);
    frame_data.extend(data);

    Ok(frame_data.freeze())
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
            return Err(Status::new(Code::Internal).with_message("Incomplete request"));
        }
        Ok(())
    }

    fn next(&mut self) -> Result<Option<Bytes>, Status> {
        if self.buf.len() <= 5 {
            return Ok(None);
        }

        let compressed = match self.buf[0] {
            1 => true,
            0 => false,
            compressed => Err(Status::new(Code::Internal)
                .with_message(format!("unsupported compressed flag: {}", compressed)))?,
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

pub(crate) fn create_decode_request_stream<T: Decoder>(
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
        while let Some(item) = stream.next().await {
            match item {
                Ok(message) => {
                    if let Ok(data) = encode_data_frame(&mut encoder, message) {
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
        while let Some(Ok(message)) = stream.next().await {
            if let Ok(data) = encode_data_frame(&mut encoder, message) {
                if sender.send_data(data).await.is_err() {
                    return;
                }
            }
        }
    });

    body.into()
}

pub(crate) fn create_decode_response_stream<T: Decoder>(
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
