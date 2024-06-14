use std::borrow::Cow;

use serde_json::Value;

use crate::{
    registry::MetaSchemaRef,
    types::{ParseError, ParseFromJSON, ParseResult, ToJSON, Type},
};

impl<T: Type> Type for sqlx::types::Json<T> {
    const IS_REQUIRED: bool = Self::RawValueType::IS_REQUIRED;

    type RawValueType = T;

    type RawElementValueType = T::RawElementValueType;

    fn name() -> Cow<'static, str> {
        Self::RawValueType::name()
    }

    fn schema_ref() -> MetaSchemaRef {
        Self::RawValueType::schema_ref()
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(&self.0)
    }

    fn raw_element_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
        self.0.raw_element_iter()
    }
}

impl<T: ParseFromJSON> ParseFromJSON for sqlx::types::Json<T> {
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        Self::RawValueType::parse_from_json(value)
            .map(sqlx::types::Json)
            .map_err(ParseError::propagate)
    }
}

impl<T: ToJSON> ToJSON for sqlx::types::Json<T> {
    fn to_json(&self) -> Option<Value> {
        self.0.to_json()
    }
}
