use poem::web::Field;
use serde_json::Value;

use crate::{
    registry::MetaSchemaRef,
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToJSON, Type, TypeName,
    },
};

impl Type for String {
    const NAME: TypeName = TypeName::Normal {
        ty: "string",
        format: None,
    };

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(Self::NAME.into()))
    }

    impl_value_type!();
}

impl ParseFromJSON for String {
    fn parse_from_json(value: Value) -> ParseResult<Self> {
        if let Value::String(value) = value {
            Ok(value)
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ParseFromParameter for String {
    fn parse_from_parameter(value: Option<&str>) -> ParseResult<Self> {
        match value {
            Some(value) => Ok(value.to_string()),
            None => Err(ParseError::expected_input()),
        }
    }
}

#[poem::async_trait]
impl ParseFromMultipartField for String {
    async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
        match field {
            Some(field) => Ok(field.text().await.map_err(ParseError::custom)?),
            None => Err(ParseError::expected_input()),
        }
    }
}

impl ToJSON for String {
    fn to_json(&self) -> Value {
        Value::String(self.clone())
    }
}

impl<'a> Type for &'a str {
    const NAME: TypeName = TypeName::Normal {
        ty: "string",
        format: None,
    };

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(Self::NAME.into()))
    }

    impl_value_type!();
}

impl<'a> ToJSON for &'a str {
    fn to_json(&self) -> Value {
        Value::String(self.to_string())
    }
}
