use std::borrow::Cow;

use poem::web::Field as PoemField;
use serde_json::Value;

use crate::{
    registry::{MetaSchemaRef, Registry},
    types::{ParseError, ParseFromJSON, ParseFromMultipartField, ParseResult, ToJSON, Type},
};

/// A JSON type for multipart field.
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct JsonField<T>(pub T);

impl<T: Type> Type for JsonField<T> {
    const IS_REQUIRED: bool = true;

    type RawValueType = T::RawValueType;

    type RawElementValueType = T::RawElementValueType;

    fn name() -> Cow<'static, str> {
        T::name()
    }

    #[inline]
    fn schema_ref() -> MetaSchemaRef {
        T::schema_ref()
    }

    fn register(registry: &mut Registry) {
        T::register(registry);
    }

    #[inline]
    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        self.0.as_raw_value()
    }

    fn raw_element_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
        self.0.raw_element_iter()
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
