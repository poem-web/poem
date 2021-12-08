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
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "string(binary)".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format("string", "binary")))
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

#[poem::async_trait]
impl ParseFromMultipartField for Binary {
    async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
        match field {
            Some(field) => Ok(Self(field.bytes().await.map_err(ParseError::custom)?)),
            None => Err(ParseError::expected_input()),
        }
    }
}
