use std::borrow::Cow;

use prost_wkt_types::Duration;
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{ParseError, ParseFromJSON, ParseFromParameter, ParseResult, ToJSON, Type},
};

impl Type for Duration {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;
    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "Protobuf_Duration".into()
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
            serde_json::from_value::<Duration>(serde_json::Value::String(value))
                .map_err(|e| ParseError::custom(e.to_string()))
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ParseFromParameter for Duration {
    fn parse_from_parameter(value: &str) -> ParseResult<Self> {
        serde_json::from_value::<Duration>(serde_json::Value::String(value.to_string()))
            .map_err(|e| ParseError::custom(e.to_string()))
    }
}

impl ToJSON for Duration {
    fn to_json(&self) -> Option<Value> {
        Some(Value::String(self.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration() {
        let duration = Duration::parse_from_json(Some(Value::String("1.5s".to_string()))).unwrap();

        assert_eq!(duration.seconds, 1);
        assert_eq!(duration.nanos, 500_000_000);

        let json = duration.to_json();
        assert_eq!(json, Some(Value::String("1.500s".to_string())));
    }

    #[test]
    fn parse_duration_precision() {
        let cases = vec![
            ("1s", (1, 0)),
            ("1.100s", (1, 100_000_000)),
            ("1.010s", (1, 10_000_000)),
            ("1.001s", (1, 1_000_000)),
            ("-1.001s", (-1, 1_000_000)),
        ];

        for (input, (expected_seconds, expected_nanos)) in cases {
            let duration =
                Duration::parse_from_json(Some(Value::String(input.to_string()))).unwrap();
            assert_eq!(duration.seconds, expected_seconds);
            assert_eq!(duration.nanos, expected_nanos);
        }
    }

    #[test]
    fn parse_invalid_duration() {
        let invalid_cases = vec![
            "",  // empty string
            "1", // missing 's'
            // "1.2.3s",                                  // multiple decimals
            "1.xs",                                    // invalid fraction
            "99999999999999999999999999999999999999s", // overflow
        ];

        for case in invalid_cases {
            let result = Duration::parse_from_json(Some(Value::String(case.to_string())));
            dbg!(&result);
            assert!(result.is_err(), "Should have failed for: {case}");
        }
    }

    #[test]
    fn parse_non_string_value() {
        let result = Duration::parse_from_json(Some(Value::Number(123.into())));
        assert!(result.is_err());
    }

    #[test]
    fn serialize_duration() {
        let duration = Duration {
            seconds: 123,
            nanos: 456_000_000,
        };
        assert_eq!(
            duration.to_json(),
            Some(Value::String("123.456s".to_string()))
        );

        let negative_duration = Duration {
            seconds: -5,
            nanos: -500_000_000,
        };
        assert_eq!(
            negative_duration.to_json(),
            Some(Value::String("-5.500s".to_string()))
        );
    }
}
