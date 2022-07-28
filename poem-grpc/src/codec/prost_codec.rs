use std::{
    io::{Error, ErrorKind, Result},
    marker::PhantomData,
};

use bytes::BytesMut;
use prost::Message;

use crate::codec::{Codec, Decoder, Encoder};

/// A [`Codec`] for Protobuf `application/grpc+proto`
#[derive(Debug)]
pub struct ProstCodec<T, U>(PhantomData<(T, U)>);

impl<T, U> Default for ProstCodec<T, U> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T, U> Codec for ProstCodec<T, U>
where
    T: Message + Send + 'static,
    U: Message + Default + Send + 'static,
{
    const CONTENT_TYPES: &'static [&'static str] = &["application/grpc", "application/grpc+proto"];

    type Encode = T;
    type Decode = U;
    type Encoder = ProstEncoder<T>;
    type Decoder = ProstDecoder<U>;

    fn encoder(&mut self) -> Self::Encoder {
        ProstEncoder(PhantomData)
    }

    fn decoder(&mut self) -> Self::Decoder {
        ProstDecoder(PhantomData)
    }
}

#[doc(hidden)]
pub struct ProstEncoder<T>(PhantomData<T>);

impl<T> Encoder for ProstEncoder<T>
where
    T: Message + Send + 'static,
{
    type Item = T;

    fn encode(&mut self, message: Self::Item, buf: &mut BytesMut) -> Result<()> {
        message
            .encode(buf)
            .map_err(|err| Error::new(ErrorKind::Other, err))
    }
}

#[doc(hidden)]
pub struct ProstDecoder<U>(PhantomData<U>);

impl<U> Decoder for ProstDecoder<U>
where
    U: Message + Default + Send + 'static,
{
    type Item = U;

    fn decode(&mut self, buf: &[u8]) -> Result<Self::Item> {
        U::decode(buf).map_err(|err| Error::new(ErrorKind::Other, err))
    }
}
