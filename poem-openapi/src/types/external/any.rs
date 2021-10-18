use serde_json::Value;

use crate::{
    registry::MetaSchemaRef,
    types::{ParseFromJSON, ParseResult, ToJSON, Type, TypeName},
};

impl Type for Value {
    const NAME: TypeName = TypeName::Normal {
        ty: "",
        format: None,
    };

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(Self::NAME.into()))
    }

    impl_value_type!();
}

impl ParseFromJSON for Value {
    fn parse_from_json(value: Value) -> ParseResult<Self> {
        Ok(value)
    }
}

impl ToJSON for Value {
    fn to_json(&self) -> Value {
        self.clone()
    }
}
