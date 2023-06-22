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

impl Type for String {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "string".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new("string")))
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
        String::is_empty(self)
    }
}

impl ParseFromJSON for String {
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        let value = value.unwrap_or_default();
        match value {
            Value::String(val) => Ok(val),
            Value::Number(num) => Ok(num.to_string()),
            Value::Bool(val) => Ok(val.to_string()),
            _ => Err(ParseError::expected_type(value)),
        }
    }
}

impl ParseFromParameter for String {
    fn parse_from_parameter(value: &str) -> ParseResult<Self> {
        Ok(value.to_string())
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
    fn to_json(&self) -> Option<Value> {
        Some(Value::String(self.clone()))
    }
}

impl ToHeader for String {
    fn to_header(&self) -> Option<HeaderValue> {
        match HeaderValue::from_str(self) {
            Ok(value) => Some(value),
            Err(_) => None,
        }
    }
}

impl<'a> Type for &'a str {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "string".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new("string")))
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(self)
    }

    fn raw_element_iter<'b>(
        &'b self,
    ) -> Box<dyn Iterator<Item = &'b Self::RawElementValueType> + 'b> {
        Box::new(self.as_raw_value().into_iter())
    }
}

impl<'a> ToJSON for &'a str {
    fn to_json(&self) -> Option<Value> {
        Some(Value::String(self.to_string()))
    }
}
