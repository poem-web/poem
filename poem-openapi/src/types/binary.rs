use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
};

use bytes::Bytes;
use poem::web::Field;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{ParseError, ParseFromMultipartField, ParseResult, Type},
};

/// Represents a binary data.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Binary<T>(pub T);

impl<T> Deref for Binary<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Binary<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Send + Sync> Type for Binary<T> {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "string(binary)".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format("string", "binary")))
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(self)
    }

    fn raw_element_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
        Box::new(self.as_raw_value().into_iter())
    }
}

#[poem::async_trait]
impl ParseFromMultipartField for Binary<Vec<u8>> {
    async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
        match field {
            Some(field) => Ok(Self(field.bytes().await.map_err(ParseError::custom)?)),
            None => Err(ParseError::expected_input()),
        }
    }
}

#[poem::async_trait]
impl ParseFromMultipartField for Binary<Bytes> {
    async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
        match field {
            Some(field) => Ok(Self(
                field
                    .bytes()
                    .await
                    .map(Into::into)
                    .map_err(ParseError::custom)?,
            )),
            None => Err(ParseError::expected_input()),
        }
    }
}
