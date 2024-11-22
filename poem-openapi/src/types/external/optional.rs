use std::borrow::Cow;

use poem::{http::HeaderValue, web::Field as PoemField};
use serde_json::Value;

use crate::{
    registry::{MetaSchemaRef, Registry},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToHeader, ToJSON, Type,
    },
};

impl<T: Type> Type for Option<T> {
    const IS_REQUIRED: bool = false;

    type RawValueType = T::RawValueType;

    type RawElementValueType = T::RawElementValueType;

    fn name() -> Cow<'static, str> {
        format!("optional_{}", T::name()).into()
    }

    fn schema_ref() -> MetaSchemaRef {
        T::schema_ref()
    }

    fn register(registry: &mut Registry) {
        T::register(registry);
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        match self {
            Some(value) => value.as_raw_value(),
            None => None,
        }
    }

    fn raw_element_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
        match self {
            Some(value) => value.raw_element_iter(),
            None => Box::new(std::iter::empty()),
        }
    }

    #[inline]
    fn is_none(&self) -> bool {
        <Option<T>>::is_none(self)
    }
}

impl<T: ParseFromJSON> ParseFromJSON for Option<T> {
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        match value.unwrap_or_default() {
            Value::Null => Ok(None),
            value => Ok(Some(
                T::parse_from_json(Some(value)).map_err(ParseError::propagate)?,
            )),
        }
    }
}

impl<T: ParseFromParameter> ParseFromParameter for Option<T> {
    fn parse_from_parameter(_value: &str) -> ParseResult<Self> {
        unreachable!()
    }

    fn parse_from_parameters<I: IntoIterator<Item = A>, A: AsRef<str>>(
        iter: I,
    ) -> ParseResult<Self> {
        let mut iter = iter.into_iter().peekable();

        if iter.peek().is_none() {
            return Ok(None);
        }

        T::parse_from_parameters(iter)
            .map_err(ParseError::propagate)
            .map(Some)
    }
}

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
    fn to_json(&self) -> Option<Value> {
        match self {
            Some(value) => value.to_json(),
            None => Some(Value::Null),
        }
    }
}

impl<T: ToHeader> ToHeader for Option<T> {
    fn to_header(&self) -> Option<HeaderValue> {
        match self {
            Some(value) => value.to_header(),
            None => None,
        }
    }
}
