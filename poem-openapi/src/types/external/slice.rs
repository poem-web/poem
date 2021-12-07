use std::borrow::Cow;

use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef, Registry},
    types::{ToJSON, Type},
};

impl<T: Type> Type for &[T] {
    fn name() -> Cow<'static, str> {
        format!("[{}]", T::name()).into()
    }

    impl_raw_value_type!();

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            items: Some(Box::new(T::schema_ref())),
            ..MetaSchema::new("array")
        }))
    }

    fn register(registry: &mut Registry) {
        T::register(registry);
    }
}

impl<T: ToJSON> ToJSON for &[T] {
    fn to_json(&self) -> Value {
        let mut values = Vec::with_capacity(self.len());
        for item in *self {
            values.push(item.to_json());
        }
        Value::Array(values)
    }
}
