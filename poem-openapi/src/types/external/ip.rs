use std::{
    borrow::Cow,
    net::{Ipv4Addr, Ipv6Addr},
};

use poem::{http::HeaderValue, web::Field};
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToHeader, ToJSON, Type,
    },
};

impl Type for Ipv4Addr {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "string(ipv4)".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format("string", "ipv4")))
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

impl ParseFromJSON for Ipv4Addr {
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        let value = value.unwrap_or_default();
        if let Value::String(value) = value {
            Ok(value.parse()?)
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ParseFromParameter for Ipv4Addr {
    fn parse_from_parameter(value: &str) -> ParseResult<Self> {
        value.parse().map_err(ParseError::custom)
    }
}

#[poem::async_trait]
impl ParseFromMultipartField for Ipv4Addr {
    async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
        match field {
            Some(field) => Ok(field.text().await?.parse()?),
            None => Err(ParseError::expected_input()),
        }
    }
}

impl ToJSON for Ipv4Addr {
    fn to_json(&self) -> Option<Value> {
        Some(Value::String(self.to_string()))
    }
}

impl ToHeader for Ipv4Addr {
    fn to_header(&self) -> Option<HeaderValue> {
        HeaderValue::from_str(&self.to_string()).ok()
    }
}

impl Type for Ipv6Addr {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "string(ipv6)".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format("string", "ipv6")))
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

impl ParseFromJSON for Ipv6Addr {
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        let value = value.unwrap_or_default();
        if let Value::String(value) = value {
            Ok(value.parse()?)
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ParseFromParameter for Ipv6Addr {
    fn parse_from_parameter(value: &str) -> ParseResult<Self> {
        value.parse().map_err(ParseError::custom)
    }
}

#[poem::async_trait]
impl ParseFromMultipartField for Ipv6Addr {
    async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
        match field {
            Some(field) => Ok(field.text().await?.parse()?),
            None => Err(ParseError::expected_input()),
        }
    }
}

impl ToJSON for Ipv6Addr {
    fn to_json(&self) -> Option<Value> {
        Some(Value::String(self.to_string()))
    }
}

impl ToHeader for Ipv6Addr {
    fn to_header(&self) -> Option<HeaderValue> {
        HeaderValue::from_str(&self.to_string()).ok()
    }
}
