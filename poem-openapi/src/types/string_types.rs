use std::{
    borrow::Cow,
    ops::{Deref, DerefMut},
};

use serde_json::Value;

use crate::{
    registry::{MetaSchema, MetaSchemaRef},
    types::{ParseError, ParseFromJSON, ParseFromParameter, ParseResult, ToJSON, Type},
};

macro_rules! impl_string_types {
    ($(#[$docs:meta])* $ty:ident, $type_name:literal, $format:literal) => {
        impl_string_types!($(#[$docs])* $ty, $type_name, $format, |_| true);
    };

    ($(#[$docs:meta])* $ty:ident, $type_name:literal, $format:literal, $validator:expr) => {
        $(#[$docs])*
        #[derive(Debug, Clone, Eq, PartialEq, Hash)]
        pub struct $ty(pub String);

        impl Deref for $ty {
            type Target = String;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl DerefMut for $ty {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl AsRef<str> for $ty {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl Type for $ty {
            const IS_REQUIRED: bool = true;

            type RawValueType = Self;

            type RawElementValueType = Self;

            fn name() -> Cow<'static, str> {
                concat!($type_name, "(", $format, ")").into()
            }

            fn schema_ref() -> MetaSchemaRef {
                MetaSchemaRef::Inline(Box::new(MetaSchema::new_with_format($type_name, $format)))
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

        impl ParseFromJSON for $ty {
            fn parse_from_json(value: Value) -> ParseResult<Self> {
                if let Value::String(value) = value {
                    let validator = $validator;
                    if !validator(&value) {
                        return Err(concat!("invalid ", $format).into());
                    }
                    Ok(Self(value))
                } else {
                    Err(ParseError::expected_type(value))
                }
            }
        }

        impl ParseFromParameter for $ty {
            fn parse_from_parameter(value: &str) -> ParseResult<Self> {
                let validator = $validator;
                if !validator(value) {
                    return Err(concat!("invalid ", $format).into());
                }
                Ok(Self(value.to_string()))
            }
        }

        impl ToJSON for $ty {
            fn to_json(&self) -> Value {
                Value::String(self.0.clone())
            }
        }
    };
}

impl_string_types!(
    /// A password type.
    ///
    /// NOTE: Its type is `string` and the format is `password`, and it does not
    /// protect the data in the memory.
    Password,
    "string",
    "password"
);

#[cfg(feature = "email")]
impl_string_types!(
    /// A email address type.
    #[cfg_attr(docsrs, doc(cfg(feature = "email")))]
    Email,
    "string",
    "email",
    email_address::EmailAddress::is_valid
);

#[cfg(feature = "hostname")]
impl_string_types!(
    /// A email address type.
    #[cfg_attr(docsrs, doc(cfg(feature = "hostname")))]
    Hostname,
    "string",
    "hostname",
    hostname_validator::is_valid
);
