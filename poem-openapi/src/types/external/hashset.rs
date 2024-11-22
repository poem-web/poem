use std::{
    borrow::Cow,
    collections::HashSet,
    hash::{BuildHasher, Hash},
};

use poem::web::Field as PoemField;
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef, Registry},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToJSON, Type,
    },
};

impl<T: Type, R: Send + Sync> Type for HashSet<T, R> {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = T::RawValueType;

    fn name() -> Cow<'static, str> {
        format!("set_{}", T::name()).into()
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
        Box::new(self.iter().filter_map(|item| item.as_raw_value()))
    }

    fn is_empty(&self) -> bool {
        HashSet::is_empty(self)
    }
}

impl<T: ParseFromJSON + Hash + Eq, R: Default + BuildHasher + Send + Sync> ParseFromJSON
    for HashSet<T, R>
{
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        let value = value.unwrap_or_default();
        match value {
            Value::Array(values) => {
                let mut res = HashSet::with_hasher(Default::default());
                for value in values {
                    res.insert(T::parse_from_json(Some(value)).map_err(ParseError::propagate)?);
                }
                Ok(res)
            }
            _ => Err(ParseError::expected_type(value)),
        }
    }
}

impl<T: ParseFromParameter + Hash + Eq, R: Send + Sync + Default + BuildHasher> ParseFromParameter
    for HashSet<T, R>
{
    fn parse_from_parameter(_value: &str) -> ParseResult<Self> {
        unreachable!()
    }

    fn parse_from_parameters<I: IntoIterator<Item = A>, A: AsRef<str>>(
        iter: I,
    ) -> ParseResult<Self> {
        let mut values = HashSet::with_hasher(Default::default());
        for s in iter {
            values.insert(
                T::parse_from_parameters(std::iter::once(s.as_ref()))
                    .map_err(ParseError::propagate)?,
            );
        }
        Ok(values)
    }
}

impl<T, R> ParseFromMultipartField for HashSet<T, R>
where
    T: ParseFromMultipartField + Hash + Eq,
    R: Send + Sync + Default + BuildHasher,
{
    async fn parse_from_multipart(field: Option<PoemField>) -> ParseResult<Self> {
        match field {
            Some(field) => {
                let item = T::parse_from_multipart(Some(field))
                    .await
                    .map_err(ParseError::propagate)?;
                let mut values = HashSet::with_hasher(Default::default());
                values.insert(item);
                Ok(values)
            }
            None => Ok(Default::default()),
        }
    }

    async fn parse_from_repeated_field(mut self, field: PoemField) -> ParseResult<Self> {
        let item = T::parse_from_multipart(Some(field))
            .await
            .map_err(ParseError::propagate)?;
        self.insert(item);
        Ok(self)
    }
}

impl<T: ToJSON, R: Send + Sync> ToJSON for HashSet<T, R> {
    fn to_json(&self) -> Option<Value> {
        let mut values = Vec::with_capacity(self.len());
        for item in self {
            if let Some(value) = item.to_json() {
                values.push(value);
            }
        }
        Some(Value::Array(values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_from_parameters() {
        let values = Vec::<i32>::parse_from_parameters(vec!["100", "200", "300"]).unwrap();
        assert_eq!(values, vec![100, 200, 300]);
    }
}
