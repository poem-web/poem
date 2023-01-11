use std::borrow::Cow;

use chrono::{DateTime, FixedOffset, Local, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use poem::web::Field;
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToJSON, Type,
    },
};

macro_rules! impl_datetime_types {
    ($ty:ty, $type_name:literal, $format:literal) => {
        impl Type for $ty {
            const IS_REQUIRED: bool = true;

            type RawValueType = Self;

            type RawElementValueType = Self;

            fn name() -> Cow<'static, str> {
                concat!($type_name, "(", $format, ")").into()
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
                let value = value.unwrap_or_default();
                if let Value::String(value) = value {
                    Ok(value.parse()?)
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

        #[poem::async_trait]
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
                Some(Value::String(self.to_rfc3339()))
            }
        }
    };
}

impl_datetime_types!(DateTime<Utc>, "string", "date-time");
impl_datetime_types!(DateTime<Local>, "string", "date-time");
impl_datetime_types!(DateTime<FixedOffset>, "string", "date-time");

macro_rules! impl_naive_datetime_types {
    ($ty:ty, $type_name:literal, $format:literal, $display:literal) => {
        impl Type for $ty {
            const IS_REQUIRED: bool = true;

            type RawValueType = Self;

            type RawElementValueType = Self;

            fn name() -> Cow<'static, str> {
                concat!($type_name, "(", $format, ")").into()
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
                let value = value.unwrap_or_default();
                if let Value::String(value) = value {
                    Ok(value.parse()?)
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

        #[poem::async_trait]
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
                Some(Value::String(format!($display, &self)))
            }
        }
    };
}

impl_naive_datetime_types!(NaiveDateTime, "string", "naive-date-time", "{:?}");
impl_naive_datetime_types!(NaiveDate, "string", "naive-date", "{}");
impl_naive_datetime_types!(NaiveTime, "string", "naive-time", "{}");

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn date_time() {
        let dt = Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(2015, 9, 18)
                .unwrap()
                .and_hms_opt(23, 56, 4)
                .unwrap(),
        );
        let value = dt.to_json();
        assert_eq!(
            value,
            Some(Value::String("2015-09-18T23:56:04+00:00".to_string()))
        );
        assert_eq!(
            DateTime::<Utc>::parse_from_json(Some(Value::String(
                "2015-09-18T23:56:04+00:00".to_string()
            )))
            .unwrap(),
            Utc.from_utc_datetime(
                &NaiveDate::from_ymd_opt(2015, 9, 18)
                    .unwrap()
                    .and_hms_opt(23, 56, 4)
                    .unwrap()
            )
        );
    }

    #[test]
    fn naive_date_time() {
        let dt = NaiveDate::from_ymd_opt(2015, 9, 18)
            .unwrap()
            .and_hms_opt(23, 56, 4)
            .unwrap();
        let value = dt.to_json();
        assert_eq!(
            value,
            Some(Value::String("2015-09-18T23:56:04".to_string()))
        );
        assert_eq!(
            NaiveDateTime::parse_from_json(Some(Value::String("2015-09-18T23:56:04".to_string())))
                .unwrap(),
            NaiveDate::from_ymd_opt(2015, 9, 18)
                .unwrap()
                .and_hms_opt(23, 56, 4)
                .unwrap()
        );
    }

    #[test]
    fn naive_date() {
        let dt = NaiveDate::from_ymd_opt(2015, 9, 18).unwrap();
        let value = dt.to_json();
        assert_eq!(value, Some(Value::String("2015-09-18".to_string())));
        assert_eq!(
            NaiveDate::parse_from_json(Some(Value::String("2015-09-18".to_string()))).unwrap(),
            NaiveDate::from_ymd_opt(2015, 9, 18).unwrap()
        );
    }

    #[test]
    fn naive_time() {
        let dt = NaiveTime::from_hms_opt(23, 56, 4).unwrap();
        let value = dt.to_json();
        assert_eq!(value, Some(Value::String("23:56:04".to_string())));
        assert_eq!(
            NaiveTime::parse_from_json(Some(Value::String("23:56:04".to_string()))).unwrap(),
            NaiveTime::from_hms_opt(23, 56, 4).unwrap()
        );
    }
}
