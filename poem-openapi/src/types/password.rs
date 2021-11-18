use std::borrow::Cow;

use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{ParseError, ParseFromJSON, ParseFromParameter, ParseResult, ToJSON, Type},
};

/// A password type.
///
/// NOTE: Its type is `string` and the format is `password`, and it does not
/// protect the data in the memory.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Password(pub String);

impl AsRef<str> for Password {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Type for Password {
    fn name() -> Cow<'static, str> {
        "string(password)".into()
    }

    impl_raw_value_type!();

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format("string", "password")))
    }
}

impl ParseFromJSON for Password {
    fn parse_from_json(value: Value) -> ParseResult<Self> {
        if let Value::String(value) = value {
            Ok(Self(value))
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ParseFromParameter for Password {
    fn parse_from_parameter(value: Option<&str>) -> ParseResult<Self> {
        match value {
            Some(value) => Ok(Self(value.to_string())),
            None => Err(ParseError::expected_input()),
        }
    }
}

impl ToJSON for Password {
    fn to_json(&self) -> Value {
        Value::String(self.0.clone())
    }
}
