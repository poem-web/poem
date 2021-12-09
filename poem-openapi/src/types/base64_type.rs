use std::borrow::Cow;

use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{ParseError, ParseFromJSON, ParseFromParameter, ParseResult, ToJSON, Type},
};

/// Represents a binary data encoded with base64.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Base64(pub Vec<u8>);

impl Type for Base64 {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "string(bytes)".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format("bytes", "string")))
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

impl ParseFromJSON for Base64 {
    fn parse_from_json(value: Value) -> ParseResult<Self> {
        if let Value::String(value) = value {
            Ok(Self(base64::decode(value)?))
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ParseFromParameter for Base64 {
    fn parse_from_parameter(value: &str) -> ParseResult<Self> {
        Ok(Self(base64::decode(value)?))
    }
}

impl ToJSON for Base64 {
    fn to_json(&self) -> Value {
        Value::String(base64::encode(&self.0))
    }
}
