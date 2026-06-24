use std::borrow::Cow;

use jiff::{
    Timestamp, Zoned,
    civil::{Date, DateTime, Time},
};
use poem::web::Field;
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToJSON, Type,
    },
};

macro_rules! impl_jiff_type {
    ($ty:ty, $type_name:literal, $format:literal) => {
        impl Type for $ty {
            const IS_REQUIRED: bool = true;

            type RawValueType = Self;

            type RawElementValueType = Self;

            fn name() -> Cow<'static, str> {
                concat!($type_name, "_", $format).into()
            }

            fn schema_ref() -> MetaSchemaRef {
                MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format($type_name, $format)))
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

        impl ParseFromJSON for $ty {
            fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
                // Distinguish "field absent" (expected_input) from "field present
                // but wrong JSON type" (expected_type)
                let value = value.ok_or_else(|| ParseError::expected_input())?;

                if let Value::String(s) = value {
                    Ok(s.parse()?)
                } else {
                    Err(ParseError::expected_type(value))
                }
            }
        }

        impl ParseFromParameter for $ty {
            fn parse_from_parameter(value: &str) -> ParseResult<Self> {
                Ok(value.parse()?)
            }
        }

        impl ParseFromMultipartField for $ty {
            async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
                match field {
                    Some(field) => Ok(field.text().await?.parse()?),
                    None => Err(ParseError::expected_input()),
                }
            }
        }

        impl ToJSON for $ty {
            fn to_json(&self) -> Option<Value> {
                Some(Value::String(self.to_string()))
            }
        }
    };
}

impl_jiff_type!(Timestamp, "string", "date-time");
impl_jiff_type!(Zoned, "string", "date-time");
impl_jiff_type!(DateTime, "string", "naive-date-time");
impl_jiff_type!(Date, "string", "naive-date");
impl_jiff_type!(Time, "string", "naive-time");

#[cfg(test)]
mod tests {
    use jiff::civil;

    use super::*;

    #[test]
    fn timestamp() {
        let ts = civil::date(2015, 9, 18)
            .at(23, 56, 4, 0)
            .in_tz("UTC")
            .unwrap()
            .timestamp();

        let value = ts.to_json();

        assert_eq!(
            value,
            Some(Value::String("2015-09-18T23:56:04Z".to_string()))
        );
        assert_eq!(
            Timestamp::parse_from_json(Some(Value::String("2015-09-18T23:56:04Z".to_string())))
                .unwrap(),
            civil::date(2015, 9, 18)
                .at(23, 56, 4, 0)
                .in_tz("UTC")
                .unwrap()
                .timestamp()
        );
    }

    #[test]
    fn zoned() {
        let zdt = civil::date(2015, 9, 18)
            .at(23, 56, 4, 0)
            .in_tz("UTC")
            .unwrap();
        let value = zdt.to_json();
        assert_eq!(
            value,
            Some(Value::String("2015-09-18T23:56:04+00:00[UTC]".to_string()))
        );
        assert_eq!(
            Zoned::parse_from_json(Some(Value::String(
                "2015-09-18T23:56:04+00:00[UTC]".to_string()
            )))
            .unwrap(),
            civil::date(2015, 9, 18)
                .at(23, 56, 4, 0)
                .in_tz("UTC")
                .unwrap()
        );
    }

    #[test]
    fn civil_datetime() {
        let dt = civil::date(2015, 9, 18).at(23, 56, 4, 0);
        let value = dt.to_json();
        assert_eq!(
            value,
            Some(Value::String("2015-09-18T23:56:04".to_string()))
        );
        assert_eq!(
            DateTime::parse_from_json(Some(Value::String("2015-09-18T23:56:04".to_string())))
                .unwrap(),
            civil::date(2015, 9, 18).at(23, 56, 4, 0)
        );
    }

    #[test]
    fn civil_date() {
        let date = civil::date(2015, 9, 18);
        let value = date.to_json();
        assert_eq!(value, Some(Value::String("2015-09-18".to_string())));
        assert_eq!(
            Date::parse_from_json(Some(Value::String("2015-09-18".to_string()))).unwrap(),
            civil::date(2015, 9, 18)
        );
    }

    #[test]
    fn civil_time() {
        let time = civil::time(23, 56, 4, 0);
        let value = time.to_json();
        assert_eq!(value, Some(Value::String("23:56:04".to_string())));
        assert_eq!(
            Time::parse_from_json(Some(Value::String("23:56:04".to_string()))).unwrap(),
            civil::time(23, 56, 4, 0)
        );
    }
}
