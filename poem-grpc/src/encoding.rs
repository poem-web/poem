use std::io::Result as IoResult;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures_util::StreamExt;
use http_body_util::{BodyExt, StreamBody};
use hyper::{HeaderMap, body::Frame};
use poem::Body;
use sync_wrapper::SyncStream;

use crate::{
    Code, CompressionEncoding, Status, Streaming,
    client::BoxBody,
    codec::{Decoder, Encoder},
};

async fn encode_data_frame<T: Encoder>(
    encoder: &mut T,
    buf: &mut BytesMut,
    message: T::Item,
    compression: Option<CompressionEncoding>,
) -> IoResult<Bytes> {
    buf.put_slice(&[compression.is_some() as u8, 0, 0, 0, 0]);

    if let Some(compression) = compression {
        let mut data = BytesMut::new();
        encoder.encode(message, &mut data)?;
        let data = compression.encode(&data).await?;
        buf.extend(data);
    } else {
        encoder.encode(message, buf)?;
    }

    let msg_len = (buf.len() - 5) as u32;
    buf.as_mut()[1..5].copy_from_slice(&msg_len.to_be_bytes());
    Ok(buf.split().freeze())
}

struct DataFrameDecoder {
    buf: BytesMut,
    compression: Option<CompressionEncoding>,
}

impl DataFrameDecoder {
    #[inline]
    fn new(compression: Option<CompressionEncoding>) -> Self {
        Self {
            buf: BytesMut::new(),
            compression,
        }
    }

    #[inline]
    fn put_slice(&mut self, data: impl AsRef<[u8]>) {
        self.buf.extend_from_slice(data.as_ref());
    }

    #[inline]
    #[allow(clippy::result_large_err)]
    fn check_incomplete(&self) -> Result<(), Status> {
        if !self.buf.is_empty() {
            return Err(Status::new(Code::Internal).with_message("incomplete request"));
        }
        Ok(())
    }

    async fn next(&mut self) -> Result<Option<Bytes>, Status> {
        if self.buf.len() < 5 {
            return Ok(None);
        }

        let compressed = match self.buf[0] {
            1 => true,
            0 => false,
            compressed => Err(Status::new(Code::Unimplemented)
                .with_message(format!("unsupported compressed flag: {compressed}")))?,
        };

        let len = u32::from_be_bytes(self.buf[1..5].try_into().unwrap()) as usize;
        if self.buf.len() >= len + 5 {
            self.buf.advance(5);
            let data = self.buf.split_to(len).freeze();

            if compressed {
                let compression = self.compression.ok_or_else(|| {
                    Status::new(Code::Unimplemented)
                        .with_message(format!("unsupported compressed flag: {compressed}"))
                })?;
                let data = compression
                    .decode(&data)
                    .await
                    .map_err(|err| Status::new(Code::Internal).with_message(err.to_string()))?;
                Ok(Some(data.into()))
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
    compression: Option<CompressionEncoding>,
) -> Streaming<T::Item> {
    let mut body: BoxBody = body.into();

    Streaming::new(async_stream::try_stream! {
        let mut frame_decoder = DataFrameDecoder::new(compression);

        loop {
            match body.frame().await.transpose().map_err(Status::from_std_error)? {
                Some(frame) => {
                    if let Ok(data) = frame.into_data() {
                        frame_decoder.put_slice(data);
                        while let Some(data) = frame_decoder.next().await? {
                            let message = decoder.decode(&data).map_err(Status::from_std_error)?;
                            yield message;
                        }
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
    compression: Option<CompressionEncoding>,
) -> Body {
    let stream = async_stream::try_stream! {
        let mut buf = BytesMut::new();

        while let Some(item) = stream.next().await {
            match item {
                Ok(message) => {
                    if let Ok(data) = encode_data_frame(&mut encoder, &mut buf, message, compression).await {
                        yield Frame::data(data);
                    }
                }
                Err(status) => {
                    yield Frame::trailers(status.to_headers());
                }
            }
        }

        yield Frame::trailers(Status::new(Code::Ok).to_headers());
    };

    BodyExt::boxed(StreamBody::new(SyncStream::new(stream))).into()
}

pub(crate) fn create_encode_request_body<T: Encoder>(
    mut encoder: T,
    mut stream: Streaming<T::Item>,
    compression: Option<CompressionEncoding>,
) -> Body {
    let stream = async_stream::try_stream! {
        let mut buf = BytesMut::new();

        while let Some(Ok(message)) = stream.next().await {
            if let Ok(data) = encode_data_frame(&mut encoder, &mut buf, message, compression).await {
                yield Frame::data(data);
            }
        }
    };

    BodyExt::boxed(StreamBody::new(SyncStream::new(stream))).into()
}

#[allow(clippy::result_large_err)]
pub(crate) fn create_decode_response_body<T: Decoder>(
    mut decoder: T,
    headers: &HeaderMap,
    body: Body,
    compression: Option<CompressionEncoding>,
) -> Result<Streaming<T::Item>, Status> {
    // check is trailers-only
    if let Some(status) = Status::from_headers(headers)? {
        return if status.is_ok() {
            Ok(Streaming::new(futures_util::stream::empty()))
        } else {
            Err(status)
        };
    }

    let mut body: BoxBody = body.into();

    Ok(Streaming::new(async_stream::try_stream! {
        let mut frame_decoder = DataFrameDecoder::new(compression);
        let mut status = None;

        while let Some(frame) = body.frame().await.transpose().map_err(Status::from_std_error)? {
            if frame.is_data() {
                let data = frame.into_data().unwrap();
                frame_decoder.put_slice(data);
                while let Some(data) = frame_decoder.next().await? {
                    let message = decoder.decode(&data).map_err(Status::from_std_error)?;
                    yield message;
                }
            } else if frame.is_trailers() {
                frame_decoder.check_incomplete()?;
                let headers = frame.into_trailers().unwrap();
                status = Some(Status::from_headers(&headers)?
                    .ok_or_else(|| Status::new(Code::Internal)
                    .with_message("missing grpc-status"))?);
                break;
            }
        }

        let status = status.ok_or_else(|| Status::new(Code::Internal).with_message("missing trailers"))?;
        if !status.is_ok() {
            Err(status)?;
        }
    }))
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use futures_util::TryStreamExt;
    use http::HeaderMap;
    use poem::Body;
    use prost::Message;

    use super::create_decode_response_body;
    use crate::codec::{Codec, ProstCodec};

    #[derive(Clone, PartialEq, Message)]
    struct TestMsg {
        #[prost(string, tag = 1)]
        value: String,
    }

    #[tokio::test]
    async fn msg_data_spans_multiple_frames() {
        // Split and encoded message into multiple frames.
        let msg = TestMsg {
            value:
                "A program is like a poem, you cannot write a poem without writing it. --- Dijkstra"
                    .into(),
        };
        let encoded = msg.encode_to_vec();
        let len = encoded.len();

        // Compression flag + u32 big endian size
        let mut buffer = vec![0];
        buffer.extend((len as u32).to_be_bytes());
        buffer.extend(encoded);

        // Split the data into multiple frames.
        let (first_frame, second_frame) = buffer.split_at(len / 2);

        let bytes_stream = futures_util::stream::iter(vec![
            Ok::<_, std::io::Error>(Bytes::from(first_frame.to_vec())),
            Ok(Bytes::from(second_frame.to_vec())),
        ]);
        let body = Body::from_bytes_stream(bytes_stream);

        let mut codec = ProstCodec::<TestMsg, TestMsg>::default();
        let mut streaming =
            create_decode_response_body(codec.decoder(), &HeaderMap::default(), body, None)
                .expect("streaming");

        let stream_msg = streaming
            .try_next()
            .await
            .expect("msg")
            .expect("one message");

        assert_eq!(msg, stream_msg);
    }
}
