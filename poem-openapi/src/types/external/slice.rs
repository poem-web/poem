use std::borrow::Cow;

use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef, Registry},
    types::{ToJSON, Type},
};

impl<T: Type> Type for &[T] {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = T::RawValueType;

    fn name() -> Cow<'static, str> {
        format!("slice_{}", T::name()).into()
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
        <[T]>::is_empty(self)
    }
}

impl<T: ToJSON> ToJSON for &[T] {
    fn to_json(&self) -> Option<Value> {
        let mut values = Vec::with_capacity(self.len());
        for item in *self {
            if let Some(value) = item.to_json() {
                values.push(value);
            }
        }
        Some(Value::Array(values))
    }
}
