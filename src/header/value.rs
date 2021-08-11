use std::convert::TryFrom;
use std::str::FromStr;

use crate::error::{Error, ErrorInvalidHeaderValue, Result};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct HeaderValue(pub(crate) http::header::HeaderValue);

impl HeaderValue {
    #[inline]
    pub(crate) fn into_inner(self) -> http::header::HeaderValue {
        self.0
    }

    #[inline]
    pub fn to_str(&self) -> Result<&str> {
        self.0
            .to_str()
            .map_err(|_| Error::internal_server_error(ErrorInvalidHeaderValue))
    }

    #[inline]
    pub fn from_static(src: &'static str) -> Self {
        Self(http::header::HeaderValue::from_static(src))
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

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
