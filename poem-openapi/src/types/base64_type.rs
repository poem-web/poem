use poem::Error;
use serde_json::Value;

use crate::{
    registry::MetaSchemaRef,
    types::{ParseError, ParseFromJSON, ParseFromParameter, ParseResult, ToJSON, Type, TypeName},
};

/// Represents a binary data encoded with base64.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Base64(pub Vec<u8>);

impl Type for Base64 {
    const NAME: TypeName = TypeName::Normal {
        ty: "string",
        format: Some("bytes"),
    };

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(Self::NAME.into()))
    }

    impl_value_type!();
}

impl ParseFromJSON for Base64 {
    fn parse_from_json(value: Value) -> ParseResult<Self> {
        if let Value::String(value) = value {
            Ok(Self(base64::decode(value).map_err(Error::bad_request)?))
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ParseFromParameter for Base64 {
    fn parse_from_parameter(value: Option<&str>) -> ParseResult<Self> {
        match value {
            Some(value) => Ok(Self(base64::decode(value)?)),
            None => Err(ParseError::expected_input()),
        }
    }
}

impl ToJSON for Base64 {
    fn to_json(&self) -> Value {
        Value::String(base64::encode(&self.0))
    }
}
