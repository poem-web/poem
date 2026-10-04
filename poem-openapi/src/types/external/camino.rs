use std::borrow::Cow;

use camino::{Utf8Path, Utf8PathBuf};
use poem::{http::HeaderValue, web::Field};
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToHeader, ToJSON, Type,
    },
};

impl Type for Utf8PathBuf {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "path".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format("string", "path")))
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

impl ParseFromJSON for Utf8PathBuf {
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        let value = value.unwrap_or_default();
        if let Value::String(value) = value {
            Ok(value.into())
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ParseFromParameter for Utf8PathBuf {
    fn parse_from_parameter(value: &str) -> ParseResult<Self> {
        Ok(Utf8Path::new(value).to_path_buf())
    }
}

impl ParseFromMultipartField for Utf8PathBuf {
    async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
        match field {
            Some(field) => Ok(field.text().await?.into()),
            None => Err(ParseError::expected_input()),
        }
    }
}

impl ToJSON for Utf8PathBuf {
    fn to_json(&self) -> Option<Value> {
        Some(Value::String(self.to_string()))
    }
}

impl ToHeader for Utf8PathBuf {
    fn to_header(&self) -> Option<HeaderValue> {
        HeaderValue::from_str(self.as_str()).ok()
    }
}

impl Type for &Utf8Path {
    const IS_REQUIRED: bool = true;

    type RawValueType = Self;

    type RawElementValueType = Self;

    fn name() -> Cow<'static, str> {
        "path".into()
    }

    fn schema_ref() -> MetaSchemaRef {
        MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format("string", "path")))
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

impl ToJSON for &Utf8Path {
    fn to_json(&self) -> Option<Value> {
        Some(Value::String(self.as_str().to_owned()))
    }
}

impl ToHeader for &Utf8Path {
    fn to_header(&self) -> Option<HeaderValue> {
        HeaderValue::from_str(self.as_str()).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_name() {
        assert_eq!(Utf8PathBuf::name(), "path");
    }

    #[test]
    fn parse_from_json_none() {
        assert_eq!(
            Utf8PathBuf::parse_from_json(None)
                .expect_err("unexpectedly succeeded in parsing `None`")
                .message(),
            ParseError::<Utf8PathBuf>::expected_type(Value::Null).message()
        );
    }

    #[test]
    fn parse_from_json_value_null() {
        assert_eq!(
            Utf8PathBuf::parse_from_json(Some(Value::Null))
                .expect_err("unexpectedly succeeded in parsing `Value::Null`")
                .message(),
            ParseError::<Utf8PathBuf>::expected_type(Value::Null).message()
        );
    }

    #[test]
    fn parse_from_json_value_string() {
        assert_eq!(
            Utf8PathBuf::parse_from_json(Some(Value::String("/a/b/c".to_owned())))
                .expect(r#"failed to parse "/a/b/c""#),
            Utf8Path::new("/a/b/c")
        );
    }

    #[test]
    fn parse_from_parameter() {
        assert_eq!(
            Utf8PathBuf::parse_from_parameter("/a/b/c").expect(r#"failed to parse "/a/b/c""#),
            Utf8Path::new("/a/b/c")
        );
    }

    #[tokio::test]
    async fn parse_from_multipart_none() {
        assert_eq!(
            Utf8PathBuf::parse_from_multipart(None)
                .await
                .expect_err("unexpectedly succeeded in parsing `None`")
                .message(),
            ParseError::<Utf8PathBuf>::expected_input().message(),
        );
    }

    #[test]
    fn to_json() {
        assert_eq!(
            Utf8Path::new("/a/b/c").to_json(),
            Some(Value::String("/a/b/c".to_owned()))
        );
    }

    #[test]
    fn to_header() {
        assert_eq!(
            Utf8Path::new("/a/b/c").to_header(),
            HeaderValue::from_str("/a/b/c").ok()
        );
    }
}
