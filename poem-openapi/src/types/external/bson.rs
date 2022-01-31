use std::{borrow::Cow};

use poem::{http::HeaderValue, web::Field};
use serde_json::Value;
use bson::oid::ObjectId;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToHeader, ToJSON, Type,
    },
};

impl Type for ObjectId {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "object(ObjectID)".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format("object", "oid")))
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(self)
    }

    fn raw_element_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
        todo!()
    }
}

impl ParseFromJSON for ObjectId {
    fn parse_from_json(value: Value) -> ParseResult<Self> {
        let v: ObjectId = serde_json::from_value(value)?;
        Ok(v)
    }
}

impl ParseFromParameter for ObjectId {
    fn parse_from_parameter(value: &str) -> ParseResult<Self> {
        ParseResult::Ok(ObjectId::parse_str(value)?)
    }
}

#[poem::async_trait]
impl ParseFromMultipartField for ObjectId {
    async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
        match field {
            Some(field) => Ok(ObjectId::parse_str(field.text().await?)?),
            None => Err(ParseError::expected_input()),
        }
    }
}

impl ToJSON for ObjectId {
    fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap()
    }
}

impl ToHeader for ObjectId {
    fn to_header(&self) -> Option<HeaderValue> {
        HeaderValue::from_str(&self.to_hex()).ok()
    }
}
