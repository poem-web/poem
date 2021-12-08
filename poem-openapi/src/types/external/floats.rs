use std::borrow::Cow;

use poem::{http::HeaderValue, web::Field};
use serde_json::{Number, Value};

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToHeader, ToJSON, Type,
    },
};

macro_rules! impl_type_for_floats {
    ($(($ty:ty, $format:literal)),*) => {
        $(
        impl Type for $ty {
            const IS_REQUIRED: bool = true;

            type RawValueType = Self;

            type RawElementValueType = Self;

            fn name() -> Cow<'static, str> {
                format!("number({})", $format).into()
            }

            fn schema_ref() -> MetaSchemaRef {
                MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format("number", $format)))
            }

            fn as_raw_value(&self) -> Option<&Self::RawValueType> {
                Some(self)
            }

            fn raw_element_iter<'a>(
                &'a self
            ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
                Box::new(self.as_raw_value().into_iter())
            }
        }

        impl ParseFromJSON for $ty {
             fn parse_from_json(value: Value) -> ParseResult<Self> {
                if let Value::Number(n) = value {
                    let n = n
                        .as_f64()
                        .ok_or_else(|| ParseError::from("invalid number"))?;
                    Ok(n as Self)
                } else {
                    Err(ParseError::expected_type(value))
                }
            }
        }

        impl ParseFromParameter for $ty {
            fn parse_from_parameter(value: Option<&str>) -> ParseResult<Self> {
                match value {
                    Some(value) => value.parse().map_err(ParseError::custom),
                    None => Err(ParseError::expected_input()),
                }
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
            fn to_json(&self) -> Value {
                Value::Number(Number::from_f64(*self as f64).unwrap())
            }
        }

        impl ToHeader for $ty {
            fn to_header(&self) -> Option<HeaderValue> {
                match HeaderValue::from_str(&format!("{}", self)) {
                    Ok(value) => Some(value),
                    Err(_) => None,
                }
            }
        }

        )*
    };
}

impl_type_for_floats!((f32, "float"), (f64, "double"));
