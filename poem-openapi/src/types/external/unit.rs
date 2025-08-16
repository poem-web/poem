use std::borrow::Cow;

use poem::{http::HeaderValue, web::Field};
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{
        ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult, ToHeader, ToJSON,
        Type,
    },
};

impl Type for () {
    const IS_REQUIRED: bool = false;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "unit".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new("unit")))
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

impl ParseFromJSON for () {
    fn parse_from_json(_: Option<Value>) -> ParseResult<Self> {
        Ok(())
    }
}

impl ParseFromParameter for () {
    fn parse_from_parameter(_: &str) -> ParseResult<Self> {
        Ok(())
    }
}

impl ParseFromMultipartField for () {
    async fn parse_from_multipart(_: Option<Field>) -> ParseResult<Self> {
        Ok(())
    }
}

impl ToJSON for () {
    fn to_json(&self) -> Option<Value> {
        Some(Value::Null)
    }
}

impl ToHeader for () {
    fn to_header(&self) -> Option<HeaderValue> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_name() {
        assert_eq!(<()>::name(), "unit");
    }

    #[test]
    fn parse_from_json_none() {
        assert_eq!(
            <()>::parse_from_json(None).expect("failed to parse 'None'"),
            ()
        );
    }

    #[test]
    fn parse_from_json_value_null() {
        assert_eq!(
            <()>::parse_from_json(Some(Value::Null)).expect("failed to parse 'Value::Null'"),
            ()
        );
    }

    #[test]
    fn parse_from_parameter() {
        assert_eq!(
            <()>::parse_from_parameter("").expect("failed to parse ''"),
            ()
        );
    }

    #[tokio::test]
    async fn parse_from_multipart_none() {
        assert_eq!(
            <()>::parse_from_multipart(None)
                .await
                .expect("failed to parse 'None'"),
            ()
        );
    }

    #[test]
    fn to_json() {
        assert_eq!(().to_json(), Some(Value::Null));
    }

    #[test]
    fn to_header() {
        assert_eq!(().to_header(), None);
    }
}
