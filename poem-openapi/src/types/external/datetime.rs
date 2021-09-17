use chrono::{DateTime, FixedOffset};
use serde_json::Value;

use crate::{
    registry::MetaSchemaRef,
    types::{ParseError, ParseFromJSON, ParseFromParameter, ParseResult, ToJSON, Type, TypeName},
};

impl Type for DateTime<FixedOffset> {
    const NAME: TypeName = TypeName::Normal {
        ty: "string",
        format: Some("data-time"),
    };

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Self::NAME.into())
    }

    impl_value_type!();
}

impl ParseFromJSON for DateTime<FixedOffset> {
    fn parse_from_json(value: Value) -> ParseResult<Self> {
        if let Value::String(value) = value {
            Ok(value.parse()?)
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ParseFromParameter for DateTime<FixedOffset> {
    fn parse_from_parameter(value: Option<&str>) -> ParseResult<Self> {
        match value {
            Some(value) => Ok(value.parse()?),
            None => Err(ParseError::expected_input()),
        }
    }
}

impl ToJSON for DateTime<FixedOffset> {
    fn to_json(&self) -> Value {
        Value::String(self.to_rfc3339())
    }
}
