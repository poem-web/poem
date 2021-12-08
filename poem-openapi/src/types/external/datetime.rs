use std::borrow::Cow;

use chrono::{DateTime, FixedOffset};
use poem::web::Field;
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToJSON, Type,
    },
};

impl Type for DateTime<FixedOffset> {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "string(date-time)".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format("string", "date-time")))
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

#[poem::async_trait]
impl ParseFromMultipartField for DateTime<FixedOffset> {
    async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
        match field {
            Some(field) => Ok(field.text().await?.parse()?),
            None => Err(ParseError::expected_input()),
        }
    }
}

impl ToJSON for DateTime<FixedOffset> {
    fn to_json(&self) -> Value {
        Value::String(self.to_rfc3339())
    }
}
