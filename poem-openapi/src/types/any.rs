use std::borrow::Cow;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{ParseError, ParseFromJSON, ParseResult, ToJSON, Type},
};

/// A any type.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Any<T>(pub T);

impl<T: Send + Sync> Type for Any<T> {
    const IS_REQUIRED: bool = true;

    type RawValueType = T;

    type RawElementValueType = T;

    fn name() -> Cow<'static, str> {
        "any".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::ANY))
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(&self.0)
    }

    fn raw_element_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
        Box::new(self.as_raw_value().into_iter())
    }
}

impl<T: DeserializeOwned + Send + Sync> ParseFromJSON for Any<T> {
    fn parse_from_json(value: Value) -> ParseResult<Self> {
        Ok(Self(
            serde_json::from_value(value).map_err(ParseError::custom)?,
        ))
    }
}

impl<T: Serialize + Send + Sync> ToJSON for Any<T> {
    fn to_json(&self) -> Value {
        serde_json::to_value(&self.0).unwrap_or_default()
    }
}

impl Type for Value {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "any".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::ANY))
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

impl ParseFromJSON for Value {
    fn parse_from_json(value: Value) -> ParseResult<Self> {
        Ok(value)
    }
}

impl ToJSON for Value {
    fn to_json(&self) -> Value {
        self.clone()
    }
}
