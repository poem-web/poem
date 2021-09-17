use poem::web::Field;

use crate::{
    registry::MetaSchemaRef,
    types::{ParseError, ParseFromMultipartField, ParseResult, Type, TypeName},
};

/// Represents a binary data.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Binary(pub Vec<u8>);

impl Type for Binary {
    const NAME: TypeName = TypeName::Normal {
        ty: "string",
        format: Some("binary"),
    };

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Self::NAME.into())
    }

    impl_value_type!();
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
