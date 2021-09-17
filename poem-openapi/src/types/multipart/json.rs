use poem::web::Field as PoemField;
use serde_json::Value;

use crate::{
    registry::MetaSchemaRef,
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseResult, ToJSON, Type, TypeName,
    },
};

/// A JSON type for multipart field.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JsonField<T>(pub T);

impl<T: Type> Type for JsonField<T> {
    const NAME: TypeName = T::NAME;

    type ValueType = T::ValueType;

    #[inline]
    fn schema_ref() -> MetaSchemaRef {
        T::schema_ref()
    }

    #[inline]
    fn as_value(&self) -> Option<&Self::ValueType> {
        self.0.as_value()
    }
}

#[poem::async_trait]
impl<T: ParseFromJSON> ParseFromMultipartField for JsonField<T> {
    async fn parse_from_multipart(field: Option<PoemField>) -> ParseResult<Self> {
        let value = match field {
            Some(field) => {
                let data = field.bytes().await.map_err(ParseError::custom)?;
                serde_json::from_slice(&data).map_err(ParseError::custom)?
            }
            None => Value::Null,
        };
        Ok(Self(
            T::parse_from_json(value).map_err(ParseError::propagate)?,
        ))
    }
}

impl<T: ToJSON> ToJSON for JsonField<T> {
    fn to_json(&self) -> Value {
        self.0.to_json()
    }
}
