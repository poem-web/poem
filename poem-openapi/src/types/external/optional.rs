use std::borrow::Cow;

use poem::web::Field as PoemField;
use serde_json::Value;

use crate::{
    registry::{MetaSchemaRef, Registry},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToJSON, Type,
    },
};

impl<T: Type> Type for Option<T> {
    const IS_REQUIRED: bool = false;

    type ValueType = T;

    fn name() -> Cow<'static, str> {
        T::name()
    }

    fn schema_ref() -> MetaSchemaRef {
        T::schema_ref()
    }

    fn register(registry: &mut Registry) {
        T::register(registry);
    }

    fn as_value(&self) -> Option<&Self::ValueType> {
        match self {
            Some(value) => Some(value),
            None => None,
        }
    }
}

impl<T: ParseFromJSON> ParseFromJSON for Option<T> {
    fn parse_from_json(value: Value) -> ParseResult<Self> {
        match value {
            Value::Null => Ok(None),
            value => Ok(Some(
                T::parse_from_json(value).map_err(ParseError::propagate)?,
            )),
        }
    }
}

impl<T: ParseFromParameter> ParseFromParameter for Option<T> {
    fn parse_from_parameter(value: Option<&str>) -> ParseResult<Self> {
        match value {
            Some(value) => T::parse_from_parameter(Some(value))
                .map_err(ParseError::propagate)
                .map(Some),
            None => Ok(None),
        }
    }
}

#[poem::async_trait]
impl<T: ParseFromMultipartField> ParseFromMultipartField for Option<T> {
    async fn parse_from_multipart(value: Option<PoemField>) -> ParseResult<Self> {
        match value {
            Some(value) => T::parse_from_multipart(Some(value))
                .await
                .map_err(ParseError::propagate)
                .map(Some),
            None => Ok(None),
        }
    }
}

impl<T: ToJSON> ToJSON for Option<T> {
    fn to_json(&self) -> Value {
        match self {
            Some(value) => value.to_json(),
            None => Value::Null,
        }
    }
}
