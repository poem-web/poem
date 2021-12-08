use std::borrow::Cow;

use poem::web::Field as PoemField;
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef, Registry},
    types::{ParseError, ParseFromJSON, ParseFromMultipartField, ParseResult, ToJSON, Type},
};

impl<T: Type> Type for Vec<T> {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = T::RawValueType;

    fn name() -> Cow<'static, str> {
        format!("[{}]", T::name()).into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            items: Some(Box::new(T::schema_ref())),
            ..MetaSchema::new("array")
        }))
    }

    fn register(registry: &mut Registry) {
        T::register(registry);
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(self)
    }

    fn raw_element_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
        Box::new(self.iter().map(|item| item.as_raw_value()).flatten())
    }
}

impl<T: ParseFromJSON> ParseFromJSON for Vec<T> {
    fn parse_from_json(value: Value) -> ParseResult<Self> {
        match value {
            Value::Array(values) => {
                let mut res = Vec::with_capacity(values.len());
                for value in values {
                    res.push(T::parse_from_json(value).map_err(ParseError::propagate)?);
                }
                Ok(res)
            }
            _ => Err(ParseError::expected_type(value)),
        }
    }
}

#[poem::async_trait]
impl<T: ParseFromMultipartField> ParseFromMultipartField for Vec<T> {
    async fn parse_from_multipart(field: Option<PoemField>) -> ParseResult<Self> {
        match field {
            Some(field) => {
                let item = T::parse_from_multipart(Some(field))
                    .await
                    .map_err(ParseError::propagate)?;
                Ok(vec![item])
            }
            None => Ok(Vec::new()),
        }
    }

    async fn parse_from_repeated_field(mut self, field: PoemField) -> ParseResult<Self> {
        let item = T::parse_from_multipart(Some(field))
            .await
            .map_err(ParseError::propagate)?;
        self.push(item);
        Ok(self)
    }
}

impl<T: ToJSON> ToJSON for Vec<T> {
    fn to_json(&self) -> Value {
        let mut values = Vec::with_capacity(self.len());
        for item in self {
            values.push(item.to_json());
        }
        Value::Array(values)
    }
}
