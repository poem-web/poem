use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use poem::{http::HeaderValue, web::Field};
use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{
        ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        ToHeader, ToJSON, Type,
    },
};

impl Type for PathBuf {
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

impl ParseFromJSON for PathBuf {
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        let value = value.unwrap_or_default();
        if let Value::String(value) = value {
            Ok(value.into())
        } else {
            Err(ParseError::expected_type(value))
        }
    }
}

impl ParseFromParameter for PathBuf {
    fn parse_from_parameter(value: &str) -> ParseResult<Self> {
        Ok(Path::new(value).to_path_buf())
    }
}

impl ParseFromMultipartField for PathBuf {
    async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
        match field {
            Some(field) => Ok(field.text().await?.into()),
            None => Err(ParseError::expected_input()),
        }
    }
}

impl ToJSON for PathBuf {
    fn to_json(&self) -> Option<Value> {
        Some(Value::String(self.to_string_lossy().into_owned()))
    }
}

impl ToHeader for PathBuf {
    fn to_header(&self) -> Option<HeaderValue> {
        HeaderValue::from_str(self.to_str()?).ok()
    }
}

impl Type for &Path {
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

impl ToJSON for &Path {
    fn to_json(&self) -> Option<Value> {
        Some(Value::String(self.to_string_lossy().into_owned()))
    }
}

impl ToHeader for &Path {
    fn to_header(&self) -> Option<HeaderValue> {
        HeaderValue::from_str(self.to_str()?).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_name() {
        assert_eq!(PathBuf::name(), "path");
    }

    #[test]
    fn parse_from_json_none() {
        assert_eq!(
            PathBuf::parse_from_json(None)
                .expect_err("unexpectedly succeeded in parsing `None`")
                .message(),
            ParseError::<PathBuf>::expected_type(Value::Null).message()
        );
    }

    #[test]
    fn parse_from_json_value_null() {
        assert_eq!(
            PathBuf::parse_from_json(Some(Value::Null))
                .expect_err("unexpectedly succeeded in parsing `Value::Null`")
                .message(),
            ParseError::<PathBuf>::expected_type(Value::Null).message()
        );
    }

    #[test]
    fn parse_from_json_value_string() {
        assert_eq!(
            PathBuf::parse_from_json(Some(Value::String("/a/b/c".to_owned())))
                .expect(r#"failed to parse "/a/b/c""#),
            Path::new("/a/b/c")
        );
    }

    #[test]
    fn parse_from_parameter() {
        assert_eq!(
            PathBuf::parse_from_parameter("/a/b/c").expect(r#"failed to parse "/a/b/c""#),
            Path::new("/a/b/c")
        );
    }

    #[tokio::test]
    async fn parse_from_multipart_none() {
        assert_eq!(
            PathBuf::parse_from_multipart(None)
                .await
                .expect_err("unexpectedly succeeded in parsing `None`")
                .message(),
            ParseError::<PathBuf>::expected_input().message(),
        );
    }

    #[test]
    fn to_json() {
        assert_eq!(
            Path::new("/a/b/c").to_path_buf().to_json(),
            Some(Value::String("/a/b/c".to_owned()))
        );
    }

    #[test]
    fn to_header() {
        assert_eq!(
            Path::new("/a/b/c").to_path_buf().to_header(),
            HeaderValue::from_str("/a/b/c").ok()
        );
    }
}
