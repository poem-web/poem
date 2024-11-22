use std::{borrow::Cow, time::Duration};

use poem::{http::HeaderValue, web::Field};
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToHeader, ToJSON, Type,
    },
};

impl Type for Duration {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "string_duration".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format("string", "duration")))
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

impl ParseFromJSON for Duration {
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        let value = value.unwrap_or_default();
        if let Value::String(value) = value {
            Ok(humantime::parse_duration(&value)?)
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ParseFromParameter for Duration {
    fn parse_from_parameter(value: &str) -> ParseResult<Self> {
        humantime::parse_duration(value).map_err(ParseError::custom)
    }
}

impl ParseFromMultipartField for Duration {
    async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
        match field {
            Some(field) => Ok(humantime::parse_duration(&field.text().await?)?),
            None => Err(ParseError::expected_input()),
        }
    }
}

impl ToJSON for Duration {
    fn to_json(&self) -> Option<Value> {
        Some(Value::String(humantime::format_duration(*self).to_string()))
    }
}

impl ToHeader for Duration {
    fn to_header(&self) -> Option<HeaderValue> {
        HeaderValue::from_str(&humantime::format_duration(*self).to_string()).ok()
    }
}
