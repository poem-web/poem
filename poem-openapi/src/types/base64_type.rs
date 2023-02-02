use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
};

use base64::engine::{general_purpose::STANDARD, Engine};
use bytes::Bytes;
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{ParseError, ParseFromJSON, ParseFromParameter, ParseResult, ToJSON, Type},
};

/// Represents a binary data encoded with base64.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Base64<T>(pub T);

impl<T> Deref for Base64<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Base64<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: AsRef<[u8]> + Send + Sync> Type for Base64<T> {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "string(bytes)".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format("string", "bytes")))
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(self)
    }

    fn raw_element_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
        Box::new(self.as_raw_value().into_iter())
    }

    fn is_empty(&self) -> bool {
        self.0.as_ref().is_empty()
    }
}

impl ParseFromJSON for Base64<Vec<u8>> {
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        let value = value.unwrap_or_default();
        if let Value::String(value) = value {
            Ok(Self(STANDARD.decode(value)?))
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ParseFromJSON for Base64<Bytes> {
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        let value = value.unwrap_or_default();
        if let Value::String(value) = value {
            Ok(Self(STANDARD.decode(value).map(Into::into)?))
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ParseFromParameter for Base64<Vec<u8>> {
    fn parse_from_parameter(value: &str) -> ParseResult<Self> {
        Ok(Self(STANDARD.decode(value)?))
    }
}

impl ParseFromParameter for Base64<Bytes> {
    fn parse_from_parameter(value: &str) -> ParseResult<Self> {
        Ok(Self(STANDARD.decode(value).map(Into::into)?))
    }
}

impl<T: AsRef<[u8]> + Send + Sync> ToJSON for Base64<T> {
    fn to_json(&self) -> Option<Value> {
        Some(Value::String(STANDARD.encode(self.0.as_ref())))
    }
}
