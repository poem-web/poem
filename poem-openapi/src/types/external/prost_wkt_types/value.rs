use std::borrow::Cow;

use prost_wkt_types::value::Kind;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{ParseError, ParseFromJSON, ParseResult, ToJSON, Type},
};

impl Type for prost_wkt_types::Value {
    const IS_REQUIRED: bool = true;

    type RawValueType = prost_wkt_types::Value;

    type RawElementValueType = prost_wkt_types::Value;

    fn name() -> Cow<'static, str> {
        "Protobuf_Value".into()
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
    fn is_empty(&self) -> bool {
        matches!(self.kind, Some(Kind::NullValue(_)) | None)
    }
}

impl ParseFromJSON for prost_wkt_types::Value {
    fn parse_from_json(value: Option<serde_json::Value>) -> ParseResult<Self> {
        let value = value.unwrap_or_default();
        serde_json::from_value(value).map_err(|e| ParseError::custom(e.to_string()))
    }
}

impl ToJSON for prost_wkt_types::Value {
    fn to_json(&self) -> Option<serde_json::Value> {
        serde_json::to_value(self).ok()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use prost_wkt_types::Value;
    use serde_json::json;

    use super::*;

    #[test]
    fn parse_from_number() {
        let value = Value::parse_from_json(Some(json!(10_f64))).unwrap();
        assert_eq!(value, Value::number(10_f64));
    }

    #[test]
    fn parse_from_string() {
        let value = Value::parse_from_json(Some(json!("Hi"))).unwrap();
        assert_eq!(value, Value::string("Hi".into()));
    }

    #[test]
    fn parse_from_bool() {
        let value = Value::parse_from_json(Some(json!(true))).unwrap();
        assert_eq!(value, Value::bool(true));
    }

    #[test]
    fn parse_from_null() {
        let value = Value::parse_from_json(Some(json!(null))).unwrap();
        assert_eq!(value, Value::null());
    }

    #[test]
    fn parse_from_struct() {
        let value = Value::parse_from_json(Some(json!({"f1": "Hello"}))).unwrap();
        assert_eq!(
            value,
            Value::pb_struct(HashMap::from([(
                "f1".into(),
                Value::string("Hello".into())
            )]))
        );
    }

    #[test]
    fn parse_from_list() {
        let value = Value::parse_from_json(Some(json!([1, 2, 3]))).unwrap();
        assert_eq!(
            value,
            Value::pb_list(vec![
                Value::number(1_f64),
                Value::number(2_f64),
                Value::number(3_f64)
            ])
        );
    }
}
