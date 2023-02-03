use std::borrow::Cow;

use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef, Registry},
    types::{ParseError, ParseFromJSON, ParseFromParameter, ParseResult, ToJSON, Type},
};

impl<T: Type, const LEN: usize> Type for [T; LEN] {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = T::RawValueType;

    fn name() -> Cow<'static, str> {
        format!("[{}]", T::name()).into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema {
            items: Some(Box::new(T::schema_ref())),
            max_length: Some(LEN),
            min_length: Some(LEN),
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
}

impl<T: ParseFromJSON, const LEN: usize> ParseFromJSON for [T; LEN] {
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        let value = value.unwrap_or_default();
        match value {
            Value::Array(values) => {
                if values.len() != LEN {
                    return Err(ParseError::custom(format!(
                        "the length of the list must be `{LEN}`."
                    )));
                }

                let mut res = Vec::with_capacity(values.len());
                for value in values {
                    res.push(T::parse_from_json(Some(value)).map_err(ParseError::propagate)?);
                }

                Ok(res.try_into().ok().unwrap())
            }
            _ => Err(ParseError::expected_type(value)),
        }
    }
}

impl<T: ParseFromParameter, const LEN: usize> ParseFromParameter for [T; LEN] {
    fn parse_from_parameter(_value: &str) -> ParseResult<Self> {
        unreachable!()
    }

    fn parse_from_parameters<I: IntoIterator<Item = A>, A: AsRef<str>>(
        iter: I,
    ) -> ParseResult<Self> {
        let mut values = Vec::new();

        for s in iter {
            values.push(
                T::parse_from_parameters(std::iter::once(s.as_ref()))
                    .map_err(ParseError::propagate)?,
            );
        }

        if values.len() != LEN {
            return Err(ParseError::custom(format!(
                "the length of the list must be `{LEN}`."
            )));
        }

        Ok(values.try_into().ok().unwrap())
    }
}

impl<T: ToJSON, const LEN: usize> ToJSON for [T; LEN] {
    fn to_json(&self) -> Option<Value> {
        let mut values = Vec::with_capacity(self.len());
        for item in self {
            if let Some(value) = item.to_json() {
                values.push(value);
            }
        }
        Some(Value::Array(values))
    }
}
