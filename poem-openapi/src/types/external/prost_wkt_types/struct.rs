use std::borrow::Cow;

use prost_wkt_types::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{ParseError, ParseFromJSON, ParseResult, ToJSON, Type},
};

impl Type for prost_wkt_types::Struct {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = <Value as Type>::RawValueType;

    fn name() -> Cow<'static, str> {
        "Protobuf_Struct".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            additional_properties: Some(Box::new(Value::schema_ref())),
            ..MetaSchema::new("object")
        }))
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(self)
    }

    fn raw_element_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
        self.fields.raw_element_iter()
    }

    fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

impl ParseFromJSON for prost_wkt_types::Struct {
    fn parse_from_json(value: Option<serde_json::Value>) -> ParseResult<Self> {
        let value = value.unwrap_or_default();
        if let serde_json::Value::Object(_) = &value {
            serde_json::from_value::<prost_wkt_types::Struct>(value)
                .map_err(|e| ParseError::custom(e.to_string()))
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ToJSON for prost_wkt_types::Struct {
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
    fn parse_from_parameters() {
        let prost_struct = prost_wkt_types::Struct::parse_from_json(Some(json!(
            {
                "f1":10_f64,
                "f2":"Hi",
                "f3":true,
                "f4":null,
                "f5": {"fa": "Hello"},
                "f6": [1,2,3]
            }
        )))
        .unwrap();

        assert_eq!(
            prost_struct.fields.get("f1").unwrap(),
            &Value::number(10_f64)
        );
        assert_eq!(
            prost_struct.fields.get("f2").unwrap(),
            &Value::string("Hi".to_string())
        );
        assert_eq!(prost_struct.fields.get("f3").unwrap(), &Value::bool(true));
        assert_eq!(prost_struct.fields.get("f4").unwrap(), &Value::null());
        assert_eq!(
            prost_struct.fields.get("f5").unwrap(),
            &Value::pb_struct(HashMap::from([(
                "fa".into(),
                Value::string("Hello".into())
            )]))
        );
        assert_eq!(
            prost_struct.fields.get("f6").unwrap(),
            &Value::pb_list(vec![
                Value::number(1_f64),
                Value::number(2_f64),
                Value::number(3_f64)
            ])
        );
    }
}
