use serde_json::Value;

use crate::{
    poem::web::Field,
    registry::MetaSchemaRef,
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToJSON, Type, TypeName,
    },
};

impl Type for bool {
    const NAME: TypeName = TypeName::Normal {
        ty: "boolean",
        format: None,
    };

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(Self::NAME.into()))
    }

    impl_value_type!();
}

impl ParseFromJSON for bool {
    fn parse_from_json(value: Value) -> ParseResult<Self> {
        if let Value::Bool(value) = value {
            Ok(value)
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ParseFromParameter for bool {
    fn parse_from_parameter(value: Option<&str>) -> ParseResult<Self> {
        match value {
            Some(value) => value.parse().map_err(ParseError::custom),
            None => Err(ParseError::expected_input()),
        }
    }
}

#[poem::async_trait]
impl ParseFromMultipartField for bool {
    async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
        match field {
            Some(field) => Ok(field.text().await?.parse()?),
            None => Err(ParseError::expected_input()),
        }
    }
}

impl ToJSON for bool {
    fn to_json(&self) -> Value {
        Value::Bool(*self)
    }
}
