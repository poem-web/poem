use poem::web::Field;
use serde_json::Value;

use crate::{
    registry::MetaSchemaRef,
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToJSON, Type, TypeName,
    },
};

macro_rules! impl_type_for_integers {
    ($(($ty:ty, $format:literal)),*) => {
        $(
        impl Type for $ty {
            const NAME: TypeName = TypeName::Normal {
                ty: "integer",
                format: Some($format),
            };

            fn schema_ref() -> MetaSchemaRef {
                MetaSchemaRef::Inline(Self::NAME.into())
            }

            impl_value_type!();
        }

        impl ParseFromJSON for $ty {
             fn parse_from_json(value: Value) -> ParseResult<Self> {
                if let Value::Number(n) = value {
                    let n = n
                        .as_i64()
                        .ok_or_else(|| ParseError::from("invalid integer"))?;

                    if n < Self::MIN as i64 || n > Self::MAX as i64 {
                        return Err(ParseError::from(format!(
                            "Only integers from {} to {} are accepted.",
                            Self::MIN,
                            Self::MAX
                        )));
                    }

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
                Value::Number((*self).into())
            }
        }

        )*
    };
}

impl_type_for_integers!(
    (i8, "int8"),
    (i16, "int16"),
    (i32, "int32"),
    (i64, "int64"),
    (u8, "uint8"),
    (u16, "uint16"),
    (u32, "uint32"),
    (u64, "uint64")
);
