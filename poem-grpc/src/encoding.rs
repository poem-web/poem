use std::{
    io::{Error as IoError, Result as IoResult},
    sync::{Arc, Mutex},
};

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
    let res = match encoder.encode(message, &mut data) {
        Ok(()) => compression.encode(&data).await,
        Err(err) => Err(err),
    };
    match res {
        Ok(data) => {
            buf.extend(data);
            Ok(finish_data_frame(buf))
        }
        Err(err) => {
            // Don't leave this frame's header in front of the next one.
            buf.clear();
            Err(err)
        }
    }
}

fn encode_uncompressed_data_frame<T: Encoder>(
    encoder: &mut T,
    buf: &mut BytesMut,
    message: T::Item,
) -> IoResult<Bytes> {
    buf.put_slice(&[0, 0, 0, 0, 0]);
    if let Err(err) = encoder.encode(message, buf) {
        // Don't leave this frame's header, or part of the message, in front of
        // the next one.
        buf.clear();
        return Err(err);
    }
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

/// Encodes the messages of a response, followed by the trailers carrying its
/// status.
///
/// An error from `stream`, or a message that fails to encode, ends the
/// response with that error as its status: `stream` is not read any further,
/// and nothing follows the trailers.
pub(crate) fn create_encode_response_body<T: Encoder>(
    mut encoder: T,
    mut stream: Streaming<T::Item>,
    compression: Option<CompressionEncoding>,
) -> Body {
    let stream = async_stream::stream! {
        let mut buf = BytesMut::new();
        let mut status = Status::new(Code::Ok);

        while let Some(item) = stream.next().await {
            let res = match item {
                Ok(message) => encode_data_frame(&mut encoder, &mut buf, message, compression)
                    .await
                    .map_err(Status::from_std_error),
                Err(err) => Err(err),
            };
            match res {
                Ok(data) => yield Ok::<_, IoError>(Frame::data(data)),
                Err(err) => {
                    status = err;
                    break;
                }
            }
        }

        yield Ok(Frame::trailers(status.to_headers()));
    };

    BodyExt::boxed(StreamBody::new(SyncStream::new(stream))).into()
}

/// The error that ended a streaming request before its end, shared between
/// the request body and the call so the call fails with it.
#[derive(Clone, Default)]
pub(crate) struct RequestError(Arc<Mutex<Option<Status>>>);

impl RequestError {
    fn set(&self, status: Status) {
        *self.0.lock().unwrap() = Some(status);
    }

    /// The error, if the request ended with one.
    pub(crate) fn get(&self) -> Option<Status> {
        self.0.lock().unwrap().clone()
    }
}

/// Encodes the messages of a streaming request.
///
/// An error from `stream`, or a message that fails to encode, ends the body
/// with an error, which resets the HTTP/2 stream: ending it cleanly would make
/// the server take the messages sent so far for the whole input. The error is
/// stored in `error`.
pub(crate) fn create_encode_request_body<T: Encoder>(
    mut encoder: T,
    mut stream: Streaming<T::Item>,
    compression: Option<CompressionEncoding>,
    error: RequestError,
) -> Body {
    let stream = async_stream::stream! {
        let mut buf = BytesMut::new();

        while let Some(item) = stream.next().await {
            let status = match item {
                Ok(message) => {
                    match encode_data_frame(&mut encoder, &mut buf, message, compression).await {
                        Ok(data) => {
                            yield Ok(Frame::data(data));
                            continue;
                        }
                        Err(err) => Status::from_std_error(err),
                    }
                }
                Err(status) => status,
            };
            error.set(status.clone());
            yield Err(IoError::other(status));
            break;
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
            missing_response_message()
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
            let status = Status::from_headers(&trailers)?.ok_or_else(missing_grpc_status)?;
            if !status.is_ok() {
                return Err(status);
            }
            return match result {
                Some(result) => result,
                None => {
                    frame_decoder.check_incomplete()?;
                    Err(missing_response_message())
                }
            };
        }
    }

    match result {
        Some(Err(status)) => Err(status),
        _ => Err(Status::new(Code::Internal).with_message("missing trailers")),
    }
}

// A wrong number of messages is `Internal`, as in grpc-go and grpc-java:
// `doc/statuscodes.md` says `Unimplemented`, which gRPC agreed to change
// (https://github.com/grpc/grpc-java/issues/11247), since the server may have
// handled the call already. A missing or invalid `grpc-status` is `Unknown`,
// as specified in <https://github.com/grpc/grpc/blob/master/doc/statuscodes.md>
// and <https://github.com/grpc/grpc/blob/master/doc/http-grpc-status-mapping.md>.

fn missing_response_message() -> Status {
    Status::new(Code::Internal).with_message("missing response message")
}

fn extra_response_message() -> Status {
    Status::new(Code::Internal).with_message("expected only one response message")
}

fn missing_grpc_status() -> Status {
    Status::new(Code::Unknown).with_message("missing grpc-status")
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
                    .ok_or_else(missing_grpc_status)?);
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
    use futures_util::TryStreamExt;
    use http::HeaderMap;
    use prost::Message;

    use super::*;
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

    /// Encodes an `i32` as 4 big-endian bytes, and fails on a negative one
    /// after writing part of it.
    struct I32Encoder;

    impl Encoder for I32Encoder {
        type Item = i32;

        fn encode(&mut self, message: i32, buf: &mut BytesMut) -> IoResult<()> {
            if message < 0 {
                buf.put_slice(&[0xff, 0xff]);
                return Err(IoError::other("negative"));
            }
            buf.put_i32(message);
            Ok(())
        }
    }

    fn i32_frame(value: i32) -> Bytes {
        let mut buf = BytesMut::new();
        buf.put_slice(&[0, 0, 0, 0, 4]);
        buf.put_i32(value);
        buf.freeze()
    }

    async fn frames(body: Body) -> Vec<IoResult<Frame<Bytes>>> {
        let mut body: BoxBody = body.into();
        let mut frames = vec![];
        while let Some(frame) = body.frame().await {
            frames.push(frame);
        }
        frames
    }

    /// A message that fails to encode leaves nothing of itself in the buffer
    /// shared with the next one.
    #[tokio::test]
    async fn failed_encode_leaves_no_bytes() {
        let mut buf = BytesMut::new();
        assert!(
            encode_data_frame(&mut I32Encoder, &mut buf, -1, None)
                .await
                .is_err()
        );
        assert!(buf.is_empty());
        let frame = encode_data_frame(&mut I32Encoder, &mut buf, 2, None).await;
        assert_eq!(frame.unwrap(), i32_frame(2));
    }

    /// Same when the message was to be compressed.
    #[cfg(feature = "gzip")]
    #[tokio::test]
    async fn failed_compressed_encode_leaves_no_bytes() {
        let gzip = Some(CompressionEncoding::GZIP);
        let mut buf = BytesMut::new();
        assert!(
            encode_data_frame(&mut I32Encoder, &mut buf, -1, gzip)
                .await
                .is_err()
        );
        assert!(buf.is_empty());
        let frame = encode_data_frame(&mut I32Encoder, &mut buf, 2, gzip)
            .await
            .unwrap();
        assert_eq!(frame[0], 1);
        assert_eq!(
            u32::from_be_bytes(frame[1..5].try_into().unwrap()) as usize,
            frame.len() - 5
        );
        let data = CompressionEncoding::GZIP.decode(&frame[5..]).await.unwrap();
        assert_eq!(data, 2i32.to_be_bytes());
    }

    fn grpc_status(frame: &IoResult<Frame<Bytes>>) -> (&str, Option<&str>) {
        let trailers = frame.as_ref().unwrap().trailers_ref().expect("trailers");
        (
            trailers["grpc-status"].to_str().unwrap(),
            trailers
                .get("grpc-message")
                .map(|value| value.to_str().unwrap()),
        )
    }

    /// A response ends with trailers carrying an OK status.
    #[tokio::test]
    async fn response_ends_with_ok_trailers() {
        let stream = Streaming::new(futures_util::stream::iter([Ok(1), Ok(2)]));
        let frames = frames(create_encode_response_body(I32Encoder, stream, None)).await;
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0].as_ref().unwrap().data_ref(), Some(&i32_frame(1)));
        assert_eq!(frames[1].as_ref().unwrap().data_ref(), Some(&i32_frame(2)));
        assert_eq!(grpc_status(&frames[2]), ("0", None));
    }

    /// A message that fails to encode ends a response with that error in the
    /// trailers, and nothing after them.
    #[tokio::test]
    async fn failed_encode_ends_response_with_status() {
        let stream = Streaming::new(futures_util::stream::iter([Ok(1), Ok(-1), Ok(2)]));
        let frames = frames(create_encode_response_body(I32Encoder, stream, None)).await;
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].as_ref().unwrap().data_ref(), Some(&i32_frame(1)));
        assert_eq!(grpc_status(&frames[1]), ("13", Some("negative")));
    }

    /// An error from the response stream ends a response with it in the
    /// trailers: the stream isn't polled again, and no second trailers follow.
    #[tokio::test]
    async fn stream_error_ends_response_with_status() {
        let error = Status::new(Code::NotFound).with_message("gone");
        let polled_after_error = Arc::new(Mutex::new(false));
        let stream = Streaming::new(futures_util::stream::iter([Ok(1), Err(error)]).chain(
            futures_util::stream::poll_fn({
                let polled_after_error = polled_after_error.clone();
                move |_| {
                    *polled_after_error.lock().unwrap() = true;
                    std::task::Poll::Ready(None)
                }
            }),
        ));
        let frames = frames(create_encode_response_body(I32Encoder, stream, None)).await;
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].as_ref().unwrap().data_ref(), Some(&i32_frame(1)));
        assert_eq!(grpc_status(&frames[1]), ("5", Some("gone")));
        assert!(!*polled_after_error.lock().unwrap());
    }

    /// A message that fails to encode ends a streaming request with an error.
    #[tokio::test]
    async fn failed_encode_ends_request_with_error() {
        let stream = Streaming::new(futures_util::stream::iter([Ok(1), Ok(-1), Ok(2)]));
        let error = RequestError::default();
        let frames = frames(create_encode_request_body(
            I32Encoder,
            stream,
            None,
            error.clone(),
        ))
        .await;
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].as_ref().unwrap().data_ref(), Some(&i32_frame(1)));
        assert!(frames[1].is_err());
        assert_eq!(error.get().unwrap().code(), Code::Internal);
    }
}
