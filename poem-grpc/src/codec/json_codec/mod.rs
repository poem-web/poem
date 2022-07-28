use std::{
    io::{Error, ErrorKind, Result},
    marker::PhantomData,
};

use bytes::{BufMut, BytesMut};
use serde::{de::DeserializeOwned, Serialize};

use crate::codec::{Codec, Decoder, Encoder};

mod i64string_deserializer;
mod i64string_serializer;

/// A [`Codec`] for JSON `application/grpc+json`
#[cfg_attr(docsrs, doc(cfg(feature = "json-codec")))]
#[derive(Debug)]
pub struct JsonCodec<T, U>(PhantomData<(T, U)>);

impl<T, U> Default for JsonCodec<T, U> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T, U> Codec for JsonCodec<T, U>
where
    T: Serialize + Send + 'static,
    U: DeserializeOwned + Send + 'static,
{
    const CONTENT_TYPES: &'static [&'static str] = &["application/json", "application/grpc+json"];

    type Encode = T;
    type Decode = U;
    type Encoder = JsonEncoder<T>;
    type Decoder = JsonDecoder<U>;

    fn encoder(&mut self) -> Self::Encoder {
        JsonEncoder(PhantomData)
    }

    fn decoder(&mut self) -> Self::Decoder {
        JsonDecoder(PhantomData)
    }
}

#[doc(hidden)]
pub struct JsonEncoder<T>(PhantomData<T>);

impl<T> Encoder for JsonEncoder<T>
where
    T: Serialize + Send + 'static,
{
    type Item = T;

    fn encode(&mut self, item: Self::Item, buf: &mut BytesMut) -> Result<()> {
        let mut ser = serde_json::Serializer::new(buf.writer());
        item.serialize(&mut ser)
            .map_err(|err| Error::new(ErrorKind::Other, err))
    }
}

#[doc(hidden)]
pub struct JsonDecoder<U>(PhantomData<U>);

impl<U: DeserializeOwned + Send + 'static> Decoder for JsonDecoder<U> {
    type Item = U;

    fn decode(&mut self, buf: &[u8]) -> Result<Self::Item> {
        let mut de = serde_json::Deserializer::from_slice(buf);
        U::deserialize(&mut de).map_err(|err| Error::new(ErrorKind::Other, err))
    }
}

/// A [`Codec`] for JSON `application/grpc+json` that serialize/deserialize
/// `i64` to string.
#[cfg_attr(docsrs, doc(cfg(feature = "json-codec")))]
#[derive(Debug)]
pub struct JsonI64ToStringCodec<T, U>(PhantomData<(T, U)>);

impl<T, U> Default for JsonI64ToStringCodec<T, U> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T, U> Codec for JsonI64ToStringCodec<T, U>
where
    T: Serialize + Send + 'static,
    U: DeserializeOwned + Send + 'static,
{
    const CONTENT_TYPES: &'static [&'static str] = &["application/json", "application/grpc+json"];

    type Encode = T;
    type Decode = U;
    type Encoder = JsonI64ToStringEncoder<T>;
    type Decoder = JsonI64ToStringDecoder<U>;

    fn encoder(&mut self) -> Self::Encoder {
        JsonI64ToStringEncoder(PhantomData)
    }

    fn decoder(&mut self) -> Self::Decoder {
        JsonI64ToStringDecoder(PhantomData)
    }
}

#[doc(hidden)]
pub struct JsonI64ToStringEncoder<T>(PhantomData<T>);

impl<T> Encoder for JsonI64ToStringEncoder<T>
where
    T: Serialize + Send + 'static,
{
    type Item = T;

    fn encode(&mut self, item: Self::Item, buf: &mut BytesMut) -> Result<()> {
        let mut ser = serde_json::Serializer::new(buf.writer());
        item.serialize(i64string_serializer::I64ToStringSerializer(&mut ser))
            .map_err(|err| Error::new(ErrorKind::Other, err))
    }
}

#[doc(hidden)]
pub struct JsonI64ToStringDecoder<U>(PhantomData<U>);

impl<U: DeserializeOwned + Send + 'static> Decoder for JsonI64ToStringDecoder<U> {
    type Item = U;

    fn decode(&mut self, buf: &[u8]) -> Result<Self::Item> {
        let mut de = serde_json::Deserializer::from_slice(buf);
        U::deserialize(i64string_deserializer::I64ToStringDeserializer(&mut de))
            .map_err(|err| Error::new(ErrorKind::Other, err))
    }
}
