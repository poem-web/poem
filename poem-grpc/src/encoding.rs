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
    let Some(compression) = compression else {
        return encode_uncompressed_data_frame(encoder, buf, message);
    };

    buf.put_slice(&[1, 0, 0, 0, 0]);
    let mut data = BytesMut::new();
    encoder.encode(message, &mut data)?;
    let data = compression.encode(&data).await?;
    buf.extend(data);
    Ok(finish_data_frame(buf))
}

fn encode_uncompressed_data_frame<T: Encoder>(
    encoder: &mut T,
    buf: &mut BytesMut,
    message: T::Item,
) -> IoResult<Bytes> {
    buf.put_slice(&[0, 0, 0, 0, 0]);
    encoder.encode(message, buf)?;
    Ok(finish_data_frame(buf))
}

fn finish_data_frame(buf: &mut BytesMut) -> Bytes {
    let msg_len = (buf.len() - 5) as u32;
    buf.as_mut()[1..5].copy_from_slice(&msg_len.to_be_bytes());
    buf.split().freeze()
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

/// Encodes the only message of a non-client-streaming request into a complete
/// body, so the END_STREAM flag is sent on its DATA frame instead of in a
/// separate, empty one that may reach the server after it has responded.
pub(crate) async fn create_encode_unary_request_body<T: Encoder>(
    mut encoder: T,
    message: T::Item,
    compression: Option<CompressionEncoding>,
) -> Result<Body, Status> {
    let mut buf = BytesMut::new();
    let data = match compression {
        // Boxed, so the compressor's state doesn't grow every call's future.
        Some(_) => {
            Box::pin(encode_data_frame(
                &mut encoder,
                &mut buf,
                message,
                compression,
            ))
            .await
        }
        None => encode_uncompressed_data_frame(&mut encoder, &mut buf, message),
    }
    .map_err(Status::from_std_error)?;
    Ok(Body::from_bytes(data))
}

/// How many bytes of a response body the client reads past an error to reach
/// its end. Dropping a body before its end resets the stream, and h2 closes
/// the connection after too many resets.
const MAX_DISCARDED_BYTES: usize = 64 * 1024;

/// Reads and discards the rest of `body`, up to [`MAX_DISCARDED_BYTES`].
///
/// It is not bounded in time: a peer that stalls before ending the body delays
/// the error until the call's own timeout, if any.
pub(crate) async fn discard_body(body: Body) {
    let mut body: BoxBody = body.into();
    let mut discarded = 0;
    while let Some(Ok(frame)) = body.frame().await {
        if let Ok(data) = frame.into_data() {
            discarded += data.len();
            if discarded > MAX_DISCARDED_BYTES {
                break;
            }
        }
    }
}

/// Decodes the only message of a non-server-streaming response.
///
/// The body is read up to the trailers even after an error, since the status
/// is only known from them and dropping the body early resets the stream. A
/// non-OK status in the trailers takes precedence over errors found locally.
pub(crate) async fn decode_single_response_message<T: Decoder>(
    mut decoder: T,
    headers: &HeaderMap,
    body: Body,
    compression: Option<CompressionEncoding>,
) -> Result<T::Item, Status> {
    // check is trailers-only
    if let Some(status) = Status::from_headers(headers)? {
        return Err(if status.is_ok() {
            Status::new(Code::Internal).with_message("missing response message")
        } else {
            status
        });
    }

    let mut body: BoxBody = body.into();
    let mut frame_decoder = DataFrameDecoder::new(compression);
    let mut result: Option<Result<T::Item, Status>> = None;
    let mut discarded = 0;

    while let Some(frame) = body
        .frame()
        .await
        .transpose()
        .map_err(Status::from_std_error)?
    {
        let frame = match frame.into_data() {
            Ok(data) => {
                if result.is_some() {
                    if matches!(result, Some(Ok(_))) && !data.is_empty() {
                        result = Some(Err(extra_response_message()));
                    }
                    discarded += data.len();
                    if discarded > MAX_DISCARDED_BYTES {
                        break;
                    }
                    continue;
                }
                frame_decoder.put_slice(data);
                result = match frame_decoder.next().await {
                    // Bytes after the message: a second, maybe partial, one.
                    Ok(Some(_)) if !frame_decoder.buf.is_empty() => {
                        Some(Err(extra_response_message()))
                    }
                    Ok(Some(data)) => Some(decoder.decode(&data).map_err(Status::from_std_error)),
                    Ok(None) => None,
                    Err(status) => Some(Err(status)),
                };
                continue;
            }
            Err(frame) => frame,
        };

        if let Ok(trailers) = frame.into_trailers() {
            let status = Status::from_headers(&trailers)?
                .ok_or_else(|| Status::new(Code::Internal).with_message("missing grpc-status"))?;
            if !status.is_ok() {
                return Err(status);
            }
            return match result {
                Some(result) => result,
                None => {
                    frame_decoder.check_incomplete()?;
                    Err(Status::new(Code::Internal).with_message("missing response message"))
                }
            };
        }
    }

    match result {
        Some(Err(status)) => Err(status),
        _ => Err(Status::new(Code::Internal).with_message("missing trailers")),
    }
}

fn extra_response_message() -> Status {
    Status::new(Code::Internal).with_message("expected only one response message")
}

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
