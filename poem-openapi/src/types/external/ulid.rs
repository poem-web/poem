use std::borrow::Cow;

use poem::{http::HeaderValue, web::Field};
use serde_json::Value;
use ulid::Ulid;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToHeader, ToJSON, Type,
    },
};

impl Type for Ulid {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "string_ulid".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format("string", "ulid")))
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

impl ParseFromJSON for Ulid {
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        let value = value.unwrap_or_default();
        if let Value::String(value) = value {
            Ok(value.parse()?)
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ParseFromParameter for Ulid {
    fn parse_from_parameter(value: &str) -> ParseResult<Self> {
        value.parse().map_err(ParseError::custom)
    }
}

impl ParseFromMultipartField for Ulid {
    async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
        match field {
            Some(field) => Ok(field.text().await?.parse()?),
            None => Err(ParseError::expected_input()),
        }
    }
}

impl ToJSON for Ulid {
    fn to_json(&self) -> Option<Value> {
        Some(Value::String(self.to_string()))
    }
}

impl ToHeader for Ulid {
    fn to_header(&self) -> Option<HeaderValue> {
        HeaderValue::from_str(&self.to_string()).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_name() {
        assert_eq!(Ulid::name(), "string_ulid");
    }

    #[test]
    fn parse_from_json_none() {
        assert_eq!(
            Ulid::parse_from_json(None)
                .expect_err("unexpectedly succeeded in parsing `None`")
                .message(),
            ParseError::<Ulid>::expected_type(Value::Null).message()
        );
    }

    #[test]
    fn parse_from_json_value_null() {
        assert_eq!(
            Ulid::parse_from_json(Some(Value::Null))
                .expect_err("unexpectedly succeeded in parsing `Value::Null`")
                .message(),
            ParseError::<Ulid>::expected_type(Value::Null).message()
        );
    }

    #[test]
    fn parse_from_json_value_string() {
        let ulid = Ulid::new();

        assert_eq!(
            Ulid::parse_from_json(Some(Value::String(ulid.to_string())))
                .expect("failed to parse ulid"),
            ulid
        );
    }

    #[test]
    fn parse_from_parameter() {
        let ulid = Ulid::new();

        assert_eq!(
            Ulid::parse_from_parameter(ulid.to_string().as_str()).expect("failed to parse ulid"),
            ulid
        );
    }

    #[tokio::test]
    async fn parse_from_multipart_none() {
        assert_eq!(
            Ulid::parse_from_multipart(None)
                .await
                .expect_err("unexpectedly succeeded in parsing `None`")
                .message(),
            ParseError::<Ulid>::expected_input().message(),
        );
    }

    #[test]
    fn to_json() {
        let ulid = Ulid::new();

        assert_eq!(ulid.to_json(), Some(Value::String(ulid.to_string())));
    }

    #[test]
    fn to_header() {
        let ulid = Ulid::new();

        assert_eq!(
            ulid.to_header(),
            HeaderValue::from_str(ulid.to_string().as_str()).ok()
        );
    }
}
