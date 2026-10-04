use std::{borrow::Cow, ops::Range};

use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef, Registry},
    types::{ParseError, ParseFromJSON, ParseResult, ToJSON, Type},
};

impl<T: Type> Type for Range<T> {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        format!("range_{}", T::name()).into()
    }

    fn schema_ref() -> MetaSchemaRef {
        let endpoint_schema = if T::IS_REQUIRED {
            T::schema_ref()
        } else {
            // Optional endpoints still require a key, but allow null.
            T::schema_ref().nullable()
        };
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            description: Some("A half-open range with an inclusive start and exclusive end."),
            required: vec!["start", "end"],
            properties: vec![("start", endpoint_schema.clone()), ("end", endpoint_schema)],
            // Together with the required keys, this rejects unknown fields
            // without needing a boolean additionalProperties schema.
            max_properties: Some(2),
            ..MetaSchema::new("object")
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
        Box::new(self.as_raw_value().into_iter())
    }
}

impl<T: ParseFromJSON> ParseFromJSON for Range<T> {
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        let value = value.ok_or_else(ParseError::expected_input)?;
        let Value::Object(mut object) = value else {
            return Err(ParseError::expected_type(value));
        };
        let start = object
            .remove("start")
            .ok_or_else(|| ParseError::custom("missing field `start`"))?;
        let end = object
            .remove("end")
            .ok_or_else(|| ParseError::custom("missing field `end`"))?;
        if let Some(name) = object.keys().next() {
            return Err(ParseError::custom(format!("unknown field `{name}`")));
        }
        let start = T::parse_from_json(Some(start))
            .map_err(|err| ParseError::custom(format!("field `start`: {}", err.message())))?;
        let end = T::parse_from_json(Some(end))
            .map_err(|err| ParseError::custom(format!("field `end`: {}", err.message())))?;
        Ok(start..end)
    }
}

impl<T: ToJSON> ToJSON for Range<T> {
    fn to_json(&self) -> Option<Value> {
        Some(serde_json::json!({
            "start": self.start.to_json(),
            "end": self.end.to_json(),
        }))
    }
}
