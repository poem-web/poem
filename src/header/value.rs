use std::convert::TryFrom;
use std::str::FromStr;

use crate::error::{Error, ErrorInvalidHeaderValue, Result};

/// Represents an HTTP header field value.
///
/// In practice, HTTP header field values are usually valid ASCII. However, the
/// HTTP spec allows for a header value to contain opaque bytes as well. In this
/// case, the header field value is not able to be represented as a string.
///
/// To handle this, the `HeaderValue` is useable as a type and can be compared
/// with strings and implements `Debug`. A `to_str` fn is provided that returns
/// an `Err` if the header value contains non visible ascii characters.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct HeaderValue(pub(crate) http::header::HeaderValue);

impl HeaderValue {
    /// Converts a [`HeaderValue`] to a byte slice.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Yields a &str slice if the HeaderValue only contains visible ASCII chars.
    ///
    /// This function will perform a scan of the header value, checking all the characters.
    #[inline]
    pub fn to_str(&self) -> Result<&str> {
        self.0
            .to_str()
            .map_err(|_| Error::internal_server_error(ErrorInvalidHeaderValue))
    }

    /// Convert a static string to a [`HeaderValue`].
    ///
    /// This function will not perform any copying, however the string is checked to ensure that no
    /// invalid characters are present. Only visible ASCII characters (32-127) are permitted.
    ///
    /// # Panics
    ///
    /// This function panics if the argument contains invalid header value characters.
    #[inline]
    pub fn from_static(src: &'static str) -> Self {
        Self(http::header::HeaderValue::from_static(src))
    }

    /// Returns true if the [`HeaderValue`] has a length of zero bytes.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the length of self in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl FromStr for HeaderValue {
    type Err = Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse::<http::header::HeaderValue>().map_err(
            |_| Error::internal_server_error(ErrorInvalidHeaderValue),
        )?))
    }
}

impl TryFrom<&str> for HeaderValue {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl TryFrom<String> for HeaderValue {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

macro_rules! value_from {
    ($($ty:ty),*) => {
        $(
        impl From<$ty> for HeaderValue {
            fn from(value: $ty) -> Self {
                Self(value.into())
            }
        }
        )*
    }
}

value_from!(i16, i32, i64, u16, u32, u64, isize, usize);

impl PartialEq<str> for HeaderValue {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<String> for HeaderValue {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        self.0.eq(other)
    }
}
