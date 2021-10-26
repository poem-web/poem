//! Commonly used data types.

#[macro_use]
mod macros;

mod any;
mod base64_type;
mod binary;
mod error;
mod external;
mod password;

pub mod multipart;

use std::borrow::Cow;

pub use any::Any;
pub use base64_type::Base64;
pub use binary::Binary;
pub use error::{ParseError, ParseResult};
pub use password::Password;
use poem::web::Field as PoemField;
use serde_json::Value;

use crate::registry::{MetaSchemaRef, Registry};

/// Represents a OpenAPI type.
pub trait Type: Send + Sync {
    /// If it is `true`, it means that this value is required.
    const IS_REQUIRED: bool = true;

    /// The value type of this type.
    type ValueType;

    /// Returns the name of this type
    fn name() -> Cow<'static, str>;

    /// Get schema reference of this type.
    fn schema_ref() -> MetaSchemaRef;

    /// Register this type to types registry.
    #[allow(unused_variables)]
    fn register(registry: &mut Registry) {}

    /// Get the value.
    fn as_value(&self) -> Option<&Self::ValueType>;
}

/// Represents a type that can parsing from JSON.
pub trait ParseFromJSON: Type {
    /// Parse from [`serde_json::Value`].
    fn parse_from_json(value: Value) -> ParseResult<Self>
    where
        Self: Sized;
}

/// Represents a type that can parsing from parameter. (header, query, path,
/// cookie)
pub trait ParseFromParameter: Type {
    /// Parse from parameter.
    fn parse_from_parameter(value: Option<&str>) -> ParseResult<Self>
    where
        Self: Sized;
}

/// Represents a type that can parsing from multipart.
#[poem::async_trait]
pub trait ParseFromMultipartField: Type {
    /// Parse from multipart field.
    async fn parse_from_multipart(field: Option<PoemField>) -> ParseResult<Self>
    where
        Self: Sized;

    /// Parse from repeated multipart field.
    async fn parse_from_repeated_field(self, _field: PoemField) -> ParseResult<Self>
    where
        Self: Sized,
    {
        Err(ParseError::<Self>::custom("repeated field"))
    }
}

/// Represents a type that can converted to JSON.
pub trait ToJSON: Type {
    /// Convert this value to [`serde_json::Value`].
    fn to_json(&self) -> Value;
}
