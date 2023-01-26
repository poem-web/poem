use std::{borrow::Cow, collections::HashMap, fmt::Display, hash::Hash, str::FromStr};

use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef, Registry},
    types::{ParseError, ParseFromJSON, ParseResult, ToJSON, Type},
};

impl<K, V> Type for HashMap<K, V>
where
    K: ToString + FromStr + Eq + Hash + Sync + Send,
    V: Type,
{
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = V::RawValueType;

    fn name() -> Cow<'static, str> {
        format!("map<string, {}>", V::name()).into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            additional_properties: Some(Box::new(V::schema_ref())),
            ..MetaSchema::new("object")
        }))
    }

    fn register(registry: &mut Registry) {
        V::register(registry);
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(self)
    }

    fn raw_element_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
        Box::new(self.values().filter_map(|item| item.as_raw_value()))
    }

    fn is_empty(&self) -> bool {
        HashMap::is_empty(self)
    }
}

impl<K, V> ParseFromJSON for HashMap<K, V>
where
    K: ToString + FromStr + Eq + Hash + Sync + Send,
    K::Err: Display,
    V: ParseFromJSON,
{
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        let value = value.unwrap_or_default();
        if let Value::Object(value) = value {
            let mut obj = HashMap::new();
            for (key, value) in value {
                let key = key
                    .parse()
                    .map_err(|err| ParseError::custom(format!("object key: {err}")))?;
                let value = V::parse_from_json(Some(value)).map_err(ParseError::propagate)?;
                obj.insert(key, value);
            }
            Ok(obj)
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl<K, V> ToJSON for HashMap<K, V>
where
    K: ToString + FromStr + Eq + Hash + Sync + Send,
    V: ToJSON,
{
    fn to_json(&self) -> Option<Value> {
        let mut map = serde_json::Map::new();
        for (name, value) in self {
            if let Some(value) = value.to_json() {
                map.insert(name.to_string(), value);
            }
        }
        Some(Value::Object(map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hashmap() {
        type MyObj = HashMap<String, i32>;

        assert_eq!(
            MyObj::schema_ref().unwrap_inline(),
            &MetaSchema {
                additional_properties: Some(Box::new(i32::schema_ref())),
                ..MetaSchema::new("object")
            }
        );
    }
}
