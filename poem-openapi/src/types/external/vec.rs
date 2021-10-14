use crate::{
    poem::web::Field as PoemField,
    registry::{MetaSchema, MetaSchemaRef, Registry},
    serde_json::Value,
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseResult, ToJSON, Type, TypeName,
    },
};

impl<T: Type> Type for Vec<T> {
    const NAME: TypeName = TypeName::Array(&T::NAME);

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            items: Some(Box::new(T::schema_ref())),
            ..MetaSchema::new("array")
        }))
    }

    impl_value_type!();

    fn register(registry: &mut Registry) {
        T::register(registry);
    }
}

impl<T: ParseFromJSON> ParseFromJSON for Vec<T> {
    fn parse_from_json(value: Value) -> ParseResult<Self> {
        match value {
            Value::Array(values) => {
                let mut res = Vec::with_capacity(values.len());
                for value in values {
                    res.push(T::parse_from_json(value).map_err(ParseError::propagate)?);
                }
                Ok(res)
            }
            _ => Err(ParseError::expected_type(value)),
        }
    }
}

#[poem::async_trait]
impl<T: ParseFromMultipartField> ParseFromMultipartField for Vec<T> {
    async fn parse_from_multipart(field: Option<PoemField>) -> ParseResult<Self> {
        match field {
            Some(field) => {
                let item = T::parse_from_multipart(Some(field))
                    .await
                    .map_err(ParseError::propagate)?;
                Ok(vec![item])
            }
            None => Ok(Vec::new()),
        }
    }

    async fn parse_from_repeated_field(mut self, field: PoemField) -> ParseResult<Self> {
        let item = T::parse_from_multipart(Some(field))
            .await
            .map_err(ParseError::propagate)?;
        self.push(item);
        Ok(self)
    }
}

impl<T: ToJSON> ToJSON for Vec<T> {
    fn to_json(&self) -> Value {
        let mut values = Vec::with_capacity(self.len());
        for item in self {
            values.push(item.to_json());
        }
        Value::Array(values)
    }
}
