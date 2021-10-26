use std::borrow::Cow;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{ParseError, ParseFromJSON, ParseResult, ToJSON, Type},
};

/// A any type.
pub struct Any<T>(pub T);

impl<T: Send + Sync> Type for Any<T> {
    fn name() -> Cow<'static, str> {
        "any".into()
    }

    impl_value_type!();

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::ANY))
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
