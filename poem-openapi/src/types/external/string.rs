use std::borrow::Cow;

use poem::web::Field;
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToJSON, Type,
    },
};

impl Type for String {
    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new("string")))
    }

    impl_raw_value_type!();

    fn name() -> Cow<'static, str> {
        "string".into()
    }
}

impl ParseFromJSON for String {
    fn parse_from_json(value: Value) -> ParseResult<Self> {
        if let Value::String(value) = value {
            Ok(value)
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ParseFromParameter for String {
    fn parse_from_parameter(value: Option<&str>) -> ParseResult<Self> {
        match value {
            Some(value) => Ok(value.to_string()),
            None => Err(ParseError::expected_input()),
        }
    }
}

#[poem::async_trait]
impl ParseFromMultipartField for String {
    async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
        match field {
            Some(field) => Ok(field.text().await.map_err(ParseError::custom)?),
            None => Err(ParseError::expected_input()),
        }
    }
}

impl ToJSON for String {
    fn to_json(&self) -> Value {
        Value::String(self.clone())
    }
}

impl<'a> Type for &'a str {
    fn name() -> Cow<'static, str> {
        "string".into()
    }

    impl_raw_value_type!();

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new("string")))
    }
}

impl<'a> ToJSON for &'a str {
    fn to_json(&self) -> Value {
        Value::String(self.to_string())
    }
}
