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

#[cfg(test)]
mod tests {
    use serde_json::json;
    use sqlx::types::Json;

    use super::*;

    #[test]
    fn json_wrapper_preserves_inner_type() {
        let value = Json::<Vec<i32>>::parse_from_json(Some(json!([1, 2, 3]))).unwrap();
        assert_eq!(value.0, vec![1, 2, 3]);
        assert_eq!(value.to_json(), Some(json!([1, 2, 3])));
        assert_eq!(value.as_raw_value(), Some(&value.0));
        assert_eq!(Json::<Vec<i32>>::name(), Vec::<i32>::name());
        assert_eq!(Json::<Vec<i32>>::schema_ref(), Vec::<i32>::schema_ref());
    }

    #[test]
    fn json_wrapper_propagates_parse_errors() {
        assert!(Json::<i32>::parse_from_json(Some(json!("invalid"))).is_err());
    }
}
