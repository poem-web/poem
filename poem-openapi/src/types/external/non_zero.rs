use std::{borrow::Cow, num::NonZero};

use poem::{http::HeaderValue, web::Field};
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseFromXML,
        ParseResult, ToHeader, ToJSON, ToXML, Type,
    },
};

macro_rules! impl_type_for_non_zero_integers {
    ($(($ty:ty, $format:literal)),*) => {
        $(
        impl Type for NonZero<$ty> {
            const IS_REQUIRED: bool = true;

            type RawValueType = Self;

            type RawElementValueType = Self;

            fn name() -> Cow<'static, str> {
                format!("non_zero_integer_{}", $format).into()
            }

            fn schema_ref() -> MetaSchemaRef {
                MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format("non_zero_integer", $format)))
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

        impl ParseFromJSON for NonZero<$ty> {
            fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
                let number = <$ty as ParseFromJSON>::parse_from_json(value)
                    .map_err(ParseError::propagate)?;

                Self::new(number).ok_or_else(|| ParseError::custom("Integer should not be 0."))
            }
        }

        impl ParseFromXML for NonZero<$ty> {
            fn parse_from_xml(value: Option<Value>) -> ParseResult<Self> {
                let number = <$ty as ParseFromXML>::parse_from_xml(value)
                    .map_err(ParseError::propagate)?;

                Self::new(number).ok_or_else(|| ParseError::custom("Integer should not be 0."))
            }
        }

        impl ParseFromParameter for NonZero<$ty> {
            fn parse_from_parameter(value: &str) -> ParseResult<Self> {
                let number = <$ty as ParseFromParameter>::parse_from_parameter(value)
                    .map_err(ParseError::propagate)?;

                Self::new(number).ok_or_else(|| ParseError::custom("Integer should not be 0."))
            }
        }

        impl ParseFromMultipartField for NonZero<$ty> {
            async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
                let number = <$ty as ParseFromMultipartField>::parse_from_multipart(field)
                    .await
                    .map_err(ParseError::propagate)?;

                Self::new(number).ok_or_else(|| ParseError::custom("Integer should not be 0."))
            }
        }

        impl ToJSON for NonZero<$ty> {
            fn to_json(&self) -> Option<Value> {
                self.get().to_json()
            }
        }

        impl ToXML for NonZero<$ty> {
            fn to_xml(&self) -> Option<Value> {
                self.get().to_xml()
            }
        }

        impl ToHeader for NonZero<$ty> {
            fn to_header(&self) -> Option<HeaderValue> {
                self.get().to_header()
            }
        }

        )*
    };
}

macro_rules! impl_type_for_non_zero_unsigneds {
    ($(($ty:ty, $format:literal, $max:expr)),*) => {
        $(
        impl Type for NonZero<$ty> {
            const IS_REQUIRED: bool = true;

            type RawValueType = Self;

            type RawElementValueType = Self;

            fn name() -> Cow<'static, str> {
                format!("non_zero_integer({})", $format).into()
            }

            fn schema_ref() -> MetaSchemaRef {
                MetaSchemaRef::Inline(Box::new(MetaSchema {
                    ty: "integer",
                    format: Some($format),
                    minimum: Some(0.0),
                    exclusive_minimum: Some(true),
                    maximum: Some($max),
                    ..MetaSchema::ANY
                }))
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

        impl ParseFromJSON for NonZero<$ty> {
            fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
                let number = <$ty as ParseFromJSON>::parse_from_json(value)
                    .map_err(ParseError::propagate)?;

                Self::new(number).ok_or_else(|| ParseError::custom("Integer should not be 0."))
            }
        }

        impl ParseFromParameter for NonZero<$ty> {
            fn parse_from_parameter(value: &str) -> ParseResult<Self> {
                let number = <$ty as ParseFromParameter>::parse_from_parameter(value)
                    .map_err(ParseError::propagate)?;

                Self::new(number).ok_or_else(|| ParseError::custom("Integer should not be 0."))
            }
        }

        impl ParseFromMultipartField for NonZero<$ty> {
            async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
                let number = <$ty as ParseFromMultipartField>::parse_from_multipart(field)
                    .await
                    .map_err(ParseError::propagate)?;

                Self::new(number).ok_or_else(|| ParseError::custom("Integer should not be 0."))
            }
        }

        impl ToJSON for NonZero<$ty> {
            fn to_json(&self) -> Option<Value> {
                self.get().to_json()
            }
        }

        impl ToHeader for NonZero<$ty> {
            fn to_header(&self) -> Option<HeaderValue> {
                self.get().to_header()
            }
        }

        )*
    };
}

impl_type_for_non_zero_integers!((i8, "int8"), (i16, "int16"), (i32, "int32"), (i64, "int64"));

impl_type_for_non_zero_unsigneds!(
    (u8, "int32", u8::MAX as f64),
    (u16, "int32", u16::MAX as f64),
    (u32, "int64", u32::MAX as f64),
    (u64, "int64", u64::MAX as f64),
    (usize, "int64", u64::MAX as f64)
);
