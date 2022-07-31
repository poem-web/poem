//! Commonly used data types.

mod any;
mod base64_type;
mod binary;
mod error;
mod external;
mod maybe_undefined;
mod string_types;

pub mod multipart;

use std::{borrow::Cow, sync::Arc};

pub use any::Any;
pub use base64_type::Base64;
pub use binary::Binary;
pub use error::{ParseError, ParseResult};
pub use maybe_undefined::MaybeUndefined;
use poem::{http::HeaderValue, web::Field as PoemField};
use serde_json::Value;
#[cfg(feature = "email")]
pub use string_types::Email;
#[cfg(feature = "hostname")]
pub use string_types::Hostname;
pub use string_types::Password;

use crate::registry::{MetaSchemaRef, Registry};

/// Represents a OpenAPI type.
pub trait Type: Send + Sync {
    /// If it is `true`, it means that this type is required.
    const IS_REQUIRED: bool;

    /// The raw type used for validator.
    ///
    /// Usually it is `Self`, but the wrapper type is its internal type.
    ///
    /// For example:
    ///
    /// `i32::RawValueType` is `i32`
    /// `Option<i32>::RawValueType` is `i32`.
    type RawValueType;

    /// The raw element type used for validator.
    type RawElementValueType;

    /// Returns the name of this type
    fn name() -> Cow<'static, str>;

    /// Get schema reference of this type.
    fn schema_ref() -> MetaSchemaRef;

    /// Register this type to types registry.
    #[allow(unused_variables)]
    fn register(registry: &mut Registry) {}

    /// Returns a reference to the raw value.
    fn as_raw_value(&self) -> Option<&Self::RawValueType>;

    /// Returns an iterator for traversing the elements.
    fn raw_element_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a>;

    /// Returns `true` if this value is empty.
    ///
    /// If the object's field has the `skip_serializing_if_is_empty` attribute,
    /// call this method to test that the value is empty.
    #[inline]
    fn is_empty(&self) -> bool {
        false
    }

    /// Returns `true` if this value is none.
    ///
    /// If the object's field has the `skip_serializing_if_is_none` attribute,
    /// call this method to test that the value is none.
    #[inline]
    fn is_none(&self) -> bool {
        false
    }
}

/// Represents a type that can parsing from JSON.
pub trait ParseFromJSON: Sized + Type {
    /// Parse from [`serde_json::Value`].
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self>;

    /// Parse from JSON string.
    fn parse_from_json_string(s: &str) -> ParseResult<Self> {
        let value = serde_json::from_str(s).map_err(|err| ParseError::custom(err.to_string()))?;
        Self::parse_from_json(value)
    }
}

/// Represents a type that can parsing from XML.
pub trait ParseFromXML: Sized + Type {
    /// Parse from [`serde_json::Value`].
    fn parse_from_xml(value: Option<Value>) -> ParseResult<Self>;

    /// Parse from XML string.
    fn parse_from_xml_string(s: &str) -> ParseResult<Self> {
        let value =
            quick_xml::de::from_str(s).map_err(|err| ParseError::custom(err.to_string()))?;
        Self::parse_from_xml(value)
    }
}

/// Represents a type that can parsing from parameter. (header, query, path,
/// cookie)
pub trait ParseFromParameter: Sized + Type {
    /// Parse from parameter.
    fn parse_from_parameter(value: &str) -> ParseResult<Self>;

    /// Parse from multiple parameters.
    fn parse_from_parameters<I: IntoIterator<Item = A>, A: AsRef<str>>(
        iter: I,
    ) -> ParseResult<Self> {
        let mut iter = iter.into_iter();
        match iter.next().as_ref().map(|item| item.as_ref()) {
            Some(value) => Self::parse_from_parameter(value),
            None => Err(ParseError::expected_input()),
        }
    }
}

/// Represents a type that can parsing from multipart.
#[poem::async_trait]
pub trait ParseFromMultipartField: Sized + Type {
    /// Parse from multipart field.
    async fn parse_from_multipart(field: Option<PoemField>) -> ParseResult<Self>;

    /// Parse from repeated multipart field.
    async fn parse_from_repeated_field(self, _field: PoemField) -> ParseResult<Self> {
        Err(ParseError::<Self>::custom("repeated field"))
    }
}

/// Represents a type that can converted to JSON value.
pub trait ToJSON: Type {
    /// Convert this value to [`Value`].
    fn to_json(&self) -> Option<Value>;

    /// Convert this value to JSON string.
    fn to_json_string(&self) -> String {
        serde_json::to_string(&self.to_json()).unwrap_or_default()
    }
}

/// Represents a type that can converted to XML value.
pub trait ToXML: Type {
    /// Convert this value to [`Value`].
    fn to_xml(&self) -> Option<Value>;

    /// Convert this value to JSON string.
    fn to_xml_string(&self) -> String {
        quick_xml::se::to_string(&self.to_xml()).unwrap_or_default()
    }
}

/// Represents a type that can converted to HTTP header.
pub trait ToHeader: Type {
    /// Convert this value to [`HeaderValue`].
    fn to_header(&self) -> Option<HeaderValue>;
}

impl<T: Type> Type for &T {
    const IS_REQUIRED: bool = T::IS_REQUIRED;

    type RawValueType = T::RawValueType;

    type RawElementValueType = T::RawElementValueType;

    fn name() -> Cow<'static, str> {
        T::name()
    }

    fn schema_ref() -> MetaSchemaRef {
        T::schema_ref()
    }

    fn register(registry: &mut Registry) {
        T::register(registry);
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        (*self).as_raw_value()
    }

    fn raw_element_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
        (*self).raw_element_iter()
    }
}

impl<T: ToJSON> ToJSON for &T {
    fn to_json(&self) -> Option<Value> {
        T::to_json(self)
    }
}

impl<T: ToXML> ToXML for &T {
    fn to_xml(&self) -> Option<Value> {
        T::to_xml(self)
    }
}

impl<T: ToHeader> ToHeader for &T {
    fn to_header(&self) -> Option<HeaderValue> {
        T::to_header(self)
    }
}

impl<T: Type> Type for Arc<T> {
    const IS_REQUIRED: bool = T::IS_REQUIRED;

    type RawValueType = T::RawValueType;

    type RawElementValueType = T::RawElementValueType;

    fn name() -> Cow<'static, str> {
        T::name()
    }

    fn schema_ref() -> MetaSchemaRef {
        T::schema_ref()
    }

    fn register(registry: &mut Registry) {
        T::register(registry);
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        self.as_ref().as_raw_value()
    }

    fn raw_element_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
        self.as_ref().raw_element_iter()
    }
}

impl<T: ParseFromJSON> ParseFromJSON for Arc<T> {
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        T::parse_from_json(value)
            .map_err(ParseError::propagate)
            .map(Arc::new)
    }
}

impl<T: ParseFromXML> ParseFromXML for Arc<T> {
    fn parse_from_xml(value: Option<Value>) -> ParseResult<Self> {
        T::parse_from_xml(value)
            .map_err(ParseError::propagate)
            .map(Arc::new)
    }
}

impl<T: ParseFromParameter> ParseFromParameter for Arc<T> {
    fn parse_from_parameter(_value: &str) -> ParseResult<Self> {
        unreachable!()
    }

    fn parse_from_parameters<I: IntoIterator<Item = A>, A: AsRef<str>>(
        iter: I,
    ) -> ParseResult<Self> {
        T::parse_from_parameters(iter)
            .map_err(ParseError::propagate)
            .map(Arc::new)
    }
}

impl<T: ToJSON> ToJSON for Arc<T> {
    fn to_json(&self) -> Option<Value> {
        self.as_ref().to_json()
    }
}

impl<T: ToXML> ToXML for Arc<T> {
    fn to_xml(&self) -> Option<Value> {
        self.as_ref().to_xml()
    }
}

impl<T: ToHeader> ToHeader for Arc<T> {
    fn to_header(&self) -> Option<HeaderValue> {
        self.as_ref().to_header()
    }
}

impl<T: Type> Type for Box<T> {
    const IS_REQUIRED: bool = T::IS_REQUIRED;

    type RawValueType = T::RawValueType;

    type RawElementValueType = T::RawElementValueType;

    fn name() -> Cow<'static, str> {
        T::name()
    }

    fn schema_ref() -> MetaSchemaRef {
        T::schema_ref()
    }

    fn register(registry: &mut Registry) {
        T::register(registry);
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        self.as_ref().as_raw_value()
    }

    fn raw_element_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = &'a Self::RawElementValueType> + 'a> {
        self.as_ref().raw_element_iter()
    }
}

impl<T: ParseFromJSON> ParseFromJSON for Box<T> {
    fn parse_from_json(value: Option<Value>) -> ParseResult<Self> {
        T::parse_from_json(value)
            .map_err(ParseError::propagate)
            .map(Box::new)
    }
}

impl<T: ParseFromXML> ParseFromXML for Box<T> {
    fn parse_from_xml(value: Option<Value>) -> ParseResult<Self> {
        T::parse_from_xml(value)
            .map_err(ParseError::propagate)
            .map(Box::new)
    }
}

impl<T: ParseFromParameter> ParseFromParameter for Box<T> {
    fn parse_from_parameter(_value: &str) -> ParseResult<Self> {
        unreachable!()
    }

    fn parse_from_parameters<I: IntoIterator<Item = A>, A: AsRef<str>>(
        iter: I,
    ) -> ParseResult<Self> {
        T::parse_from_parameters(iter)
            .map_err(ParseError::propagate)
            .map(Box::new)
    }
}

#[poem::async_trait]
impl<T: ParseFromMultipartField> ParseFromMultipartField for Box<T> {
    async fn parse_from_multipart(field: Option<PoemField>) -> ParseResult<Self> {
        T::parse_from_multipart(field)
            .await
            .map_err(ParseError::propagate)
            .map(Box::new)
    }

    async fn parse_from_repeated_field(self, field: PoemField) -> ParseResult<Self> {
        T::parse_from_repeated_field(*self, field)
            .await
            .map_err(ParseError::propagate)
            .map(Box::new)
    }
}

impl<T: ToJSON> ToJSON for Box<T> {
    fn to_json(&self) -> Option<Value> {
        self.as_ref().to_json()
    }
}

impl<T: ToXML> ToXML for Box<T> {
    fn to_xml(&self) -> Option<Value> {
        self.as_ref().to_xml()
    }
}

impl<T: ToHeader> ToHeader for Box<T> {
    fn to_header(&self) -> Option<HeaderValue> {
        self.as_ref().to_header()
    }
}

/// Represents an example
pub trait Example {
    /// Returns the example object
    fn example() -> Self;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn arc_type() {
        assert!(Arc::<i32>::IS_REQUIRED);
        assert_eq!(Arc::<i32>::name(), "integer(int32)");
        assert_eq!(Arc::new(100).as_raw_value(), Some(&100));

        let value: Arc<i32> =
            ParseFromJSON::parse_from_json(Some(Value::Number(100.into()))).unwrap();
        assert_eq!(value, Arc::new(100));

        let value: Arc<i32> =
            ParseFromXML::parse_from_xml(Some(Value::Number(100.into()))).unwrap();
        assert_eq!(value, Arc::new(100));

        let value: Arc<i32> =
            ParseFromParameter::parse_from_parameters(std::iter::once("100")).unwrap();
        assert_eq!(value, Arc::new(100));

        assert_eq!(
            ToJSON::to_json(&Arc::new(100)),
            Some(Value::Number(100.into()))
        );

        assert_eq!(
            ToXML::to_xml(&Arc::new(100)),
            Some(Value::Number(100.into()))
        );
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn box_type() {
        assert!(Box::<i32>::IS_REQUIRED);
        assert_eq!(Box::<i32>::name(), "integer(int32)");
        assert_eq!(Box::new(100).as_raw_value(), Some(&100));

        let value: Box<i32> =
            ParseFromJSON::parse_from_json(Some(Value::Number(100.into()))).unwrap();
        assert_eq!(value, Box::new(100));

        let value: Box<i32> =
            ParseFromJSON::parse_from_json(Some(Value::Number(100.into()))).unwrap();
        assert_eq!(value, Box::new(100));

        let value: Box<i32> =
            ParseFromXML::parse_from_xml(Some(Value::Number(100.into()))).unwrap();
        assert_eq!(value, Box::new(100));

        let value: Box<i32> =
            ParseFromParameter::parse_from_parameters(std::iter::once("100")).unwrap();
        assert_eq!(value, Box::new(100));

        assert_eq!(
            ToJSON::to_json(&Box::new(100)),
            Some(Value::Number(100.into()))
        );

        assert_eq!(
            ToXML::to_xml(&Box::new(100)),
            Some(Value::Number(100.into()))
        );
    }
}
