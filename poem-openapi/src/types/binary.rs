use std::borrow::Cow;

use poem::web::Field;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{ParseError, ParseFromMultipartField, ParseResult, Type},
};

/// Represents a binary data.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Binary(pub Vec<u8>);

impl Type for Binary {
    fn name() -> Cow<'static, str> {
        "string(binary)".into()
    }

    impl_raw_value_type!();

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format("string", "binary")))
    }
}

#[poem::async_trait]
impl ParseFromMultipartField for Binary {
    async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
        match field {
            Some(field) => Ok(Self(field.bytes().await.map_err(ParseError::custom)?)),
            None => Err(ParseError::expected_input()),
        }
    }
}
