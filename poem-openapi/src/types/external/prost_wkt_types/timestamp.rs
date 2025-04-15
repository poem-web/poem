use std::borrow::Cow;

use prost_wkt_types::Timestamp;
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{ParseError, ParseFromJSON, ParseFromParameter, ParseResult, ToJSON, Type},
};

impl Type for Timestamp {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;
    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "Protobuf_Timestamp".into()
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

impl ParseFromJSON for Timestamp {
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        let value = value.unwrap_or_default();
        if let Value::String(value) = value {
            serde_json::from_value::<Timestamp>(serde_json::Value::String(value))
                .map_err(|e| ParseError::custom(e.to_string()))
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ParseFromParameter for Timestamp {
    fn parse_from_parameter(value: &str) -> ParseResult<Self> {
        serde_json::from_value::<Timestamp>(serde_json::Value::String(value.to_string()))
            .map_err(|e| ParseError::custom(e.to_string()))
    }
}

impl ToJSON for Timestamp {
    fn to_json(&self) -> Option<Value> {
        Some(Value::String(self.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_timestamp() {
        let ts =
            Timestamp::parse_from_json(Some(Value::String("1970-01-01T00:00:00Z".to_string())))
                .unwrap();

        assert_eq!(ts.seconds, 0);
        assert_eq!(ts.nanos, 0);

        let json = ts.to_json();
        assert_eq!(
            json,
            Some(Value::String("1970-01-01T00:00:00Z".to_string()))
        );
    }

    #[test]
    fn parse_timestamp_with_nanos() {
        let ts = Timestamp::parse_from_json(Some(Value::String(
            "1970-01-01T00:00:01.123456789Z".to_string(),
        )))
        .unwrap();

        assert_eq!(ts.seconds, 1);
        assert_eq!(ts.nanos, 123456789);

        let json = ts.to_json();
        assert_eq!(
            json,
            Some(Value::String("1970-01-01T00:00:01.123456789Z".to_string()))
        );
    }

    #[test]
    fn parse_invalid_timestamp() {
        let result =
            Timestamp::parse_from_json(Some(Value::String("invalid-timestamp".to_string())));
        assert!(result.is_err());
    }

    #[test]
    fn parse_non_string_value() {
        let result = Timestamp::parse_from_json(Some(Value::Number(123.into())));
        assert!(result.is_err());
    }
}
