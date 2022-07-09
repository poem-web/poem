use std::borrow::Cow;

use poem::web::Field;
use serde_json::Value;
use time::{
    format_description::well_known::Rfc3339, macros::format_description, Date, OffsetDateTime,
    PrimitiveDateTime, Time,
};

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToJSON, Type,
    },
};

impl Type for OffsetDateTime {
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

impl ParseFromJSON for OffsetDateTime {
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        let value = value.unwrap_or_default();
        if let Value::String(value) = value {
            Ok(OffsetDateTime::parse(&value, &Rfc3339)?)
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ParseFromParameter for OffsetDateTime {
    fn parse_from_parameter(value: &str) -> ParseResult<Self> {
        Ok(OffsetDateTime::parse(value, &Rfc3339)?)
    }
}

#[poem::async_trait]
impl ParseFromMultipartField for OffsetDateTime {
    async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
        match field {
            Some(field) => Ok(OffsetDateTime::parse(&field.text().await?, &Rfc3339)?),
            None => Err(ParseError::expected_input()),
        }
    }
}

impl ToJSON for OffsetDateTime {
    fn to_json(&self) -> Option<Value> {
        Some(Value::String(self.format(&Rfc3339).unwrap_or_default()))
    }
}

macro_rules! impl_naive_datetime_types {
    ($ty:ty, $type_name:literal, $format:literal, $format_description:expr) => {
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
                    Ok(<$ty>::parse(&value, $format_description)?)
                } else {
                    Err(ParseError::expected_type(value))
                }
            }
        }

        impl ParseFromParameter for $ty {
            fn parse_from_parameter(value: &str) -> ParseResult<Self> {
                Ok(<$ty>::parse(&value, $format_description)?)
            }
        }

        #[poem::async_trait]
        impl ParseFromMultipartField for $ty {
            async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
                match field {
                    Some(field) => Ok(<$ty>::parse(&field.text().await?, $format_description)?),
                    None => Err(ParseError::expected_input()),
                }
            }
        }

        impl ToJSON for $ty {
            fn to_json(&self) -> Option<Value> {
                Some(Value::String(self.format($format_description).unwrap()))
            }
        }
    };
}

impl_naive_datetime_types!(
    PrimitiveDateTime,
    "string",
    "naive-date-time",
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second]")
);
impl_naive_datetime_types!(
    Date,
    "string",
    "naive-date",
    format_description!("[year]-[month]-[day]")
);
impl_naive_datetime_types!(
    Time,
    "string",
    "naive-time",
    format_description!("[hour]:[minute]:[second]")
);
