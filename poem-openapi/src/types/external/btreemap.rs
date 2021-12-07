use std::{borrow::Cow, collections::BTreeMap, fmt::Display, str::FromStr};

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{ParseError, ParseFromJSON, ParseResult, ToJSON, Type},
};

impl<K, V> Type for BTreeMap<K, V>
where
    K: ToString + FromStr + Ord + Sync + Send,
    V: Serialize + DeserializeOwned + Send + Sync,
{
    fn name() -> Cow<'static, str> {
        "object".into()
    }

    impl_raw_value_type!();

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new("object")))
    }
}

impl<K, V> ParseFromJSON for BTreeMap<K, V>
where
    K: ToString + FromStr + Ord + Sync + Send,
    K::Err: Display,
    V: Serialize + DeserializeOwned + Send + Sync,
{
    fn parse_from_json(value: Value) -> ParseResult<Self> {
        if let Value::Object(value) = value {
            let mut obj = BTreeMap::new();
            for (key, value) in value {
                let key = key
                    .parse()
                    .map_err(|err| ParseError::custom(format!("object key: {}", err)))?;
                let value = serde_json::from_value(value)
                    .map_err(|err| ParseError::custom(format!("object value: {}", err)))?;
                obj.insert(key, value);
            }
            Ok(obj)
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl<K, V> ToJSON for BTreeMap<K, V>
where
    K: ToString + FromStr + Ord + Sync + Send,
    V: Serialize + DeserializeOwned + Send + Sync,
{
    fn to_json(&self) -> Value {
        let mut map = serde_json::Map::new();
        for (name, value) in self {
            map.insert(
                name.to_string(),
                serde_json::to_value(value).unwrap_or_default(),
            );
        }
        Value::Object(map)
    }
}
