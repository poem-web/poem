use std::borrow::Cow;

use poem::{http::HeaderValue, web::Field};
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToHeader, ToJSON, Type,
    },
};

impl Type for bool {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "boolean".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new("boolean")))
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

impl ParseFromJSON for bool {
    fn parse_from_json(value: Value) -> ParseResult<Self> {
        if let Value::Bool(value) = value {
            Ok(value)
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ParseFromParameter for bool {
    fn parse_from_parameter(value: Option<&str>) -> ParseResult<Self> {
        match value {
            Some(value) => value.parse().map_err(ParseError::custom),
            None => Err(ParseError::expected_input()),
        }
    }
}

#[poem::async_trait]
impl ParseFromMultipartField for bool {
    async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
        match field {
            Some(field) => Ok(field.text().await?.parse()?),
            None => Err(ParseError::expected_input()),
        }
    }
}

impl ToJSON for bool {
    fn to_json(&self) -> Value {
        Value::Bool(*self)
    }
}

impl ToHeader for bool {
    fn to_header(&self) -> Option<HeaderValue> {
        match self {
            true => Some(HeaderValue::from_static("true")),
            false => Some(HeaderValue::from_static("false")),
        }
    }
}
