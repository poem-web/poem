//! Metadata releated types

use std::str::FromStr;

use base64::engine::{general_purpose::STANDARD_NO_PAD, Engine};
use hyper::header::HeaderName;
use poem::http::{HeaderMap, HeaderValue};

/// A metadata map
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    pub(crate) headers: HeaderMap,
}

impl Metadata {
    /// Create an new `Metadata`
    #[inline]
    pub fn new() -> Self {
        Self {
            headers: Default::default(),
        }
    }

    /// Create an empty `Metadata` with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            headers: HeaderMap::with_capacity(capacity),
        }
    }

    /// Returns the number of entries stored in the map.
    #[inline]
    pub fn len(&self) -> usize {
        self.headers.len()
    }

    /// Returns the number of keys stored in the metadata.
    #[inline]
    pub fn keys_len(&self) -> usize {
        self.headers.keys_len()
    }

    /// Returns `true` if the metadata contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }

    /// Clears the metadata, removing all entries.
    #[inline]
    pub fn clear(&mut self) {
        self.headers.clear();
    }

    /// Returns the number of entries the map can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.headers.capacity()
    }

    /// Reserves capacity for at least additional more entries to be inserted
    /// into the `Metadata`.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.headers.reserve(additional);
    }

    /// Returns a reference to the ascii value associated with the key.
    pub fn get(&self, key: impl AsRef<str>) -> Option<&str> {
        self.headers
            .get(key.as_ref())
            .and_then(|value| value.to_str().ok())
    }

    /// Returns a decoded binary value associated with the key.
    pub fn get_bin(&self, key: impl AsRef<str>) -> Option<Vec<u8>> {
        self.headers
            .get(format!("{}-bin", key.as_ref()))
            .and_then(|value| STANDARD_NO_PAD.decode(value.as_bytes()).ok())
    }

    /// Returns a view of all ascii values associated with a key.
    #[inline]
    pub fn get_all(&self, key: impl AsRef<str>) -> GetAll<'_> {
        GetAll {
            iter: self.headers.get_all(key.as_ref()).into_iter(),
        }
    }

    /// Returns a view of all binary values associated with a key.
    #[inline]
    pub fn get_bin_all(&self, key: impl AsRef<str>) -> GetBinaryAll<'_> {
        GetBinaryAll {
            iter: self
                .headers
                .get_all(format!("{}-bin", key.as_ref()))
                .into_iter(),
        }
    }

    /// Returns `true` if the metadata contains a ascii value for the specified
    /// key.
    #[inline]
    pub fn contains_key(&self, key: impl AsRef<str>) -> bool {
        self.headers.contains_key(key.as_ref())
    }

    /// Returns `true` if the metadata contains a ascii value for the specified
    /// key.
    #[inline]
    pub fn contains_bin_key(&self, key: impl AsRef<str>) -> bool {
        self.headers.contains_key(format!("{}-bin", key.as_ref()))
    }

    /// Appends a ascii entry into the metadata.
    pub fn append(&mut self, key: impl AsRef<str>, value: impl Into<String>) {
        self.headers.append(
            HeaderName::from_str(key.as_ref()).expect("valid name"),
            HeaderValue::from_maybe_shared(value.into()).expect("valid value"),
        );
    }

    /// Appends a binary entry into the metadata.
    pub fn append_bin(&mut self, key: impl AsRef<str>, value: impl AsRef<[u8]>) {
        self.headers.append(
            HeaderName::from_str(&format!("{}-bin", key.as_ref())).expect("valid name"),
            HeaderValue::from_maybe_shared(STANDARD_NO_PAD.encode(value)).expect("valid value"),
        );
    }

    /// Inserts a ascii entry into the metadata.
    pub fn insert(&mut self, key: impl AsRef<str>, value: impl Into<String>) {
        self.headers.insert(
            HeaderName::from_str(key.as_ref()).expect("valid name"),
            HeaderValue::from_maybe_shared(value.into()).expect("valid value"),
        );
    }

    /// Inserts a binary entry into the metadata.
    pub fn insert_bin(&mut self, key: impl AsRef<str>, value: impl AsRef<[u8]>) {
        self.headers.insert(
            HeaderName::from_str(&format!("{}-bin", key.as_ref())).expect("valid name"),
            HeaderValue::from_maybe_shared(STANDARD_NO_PAD.encode(value)).expect("valid value"),
        );
    }
}

/// A view to all ascii values stored in a single entry.
pub struct GetAll<'a> {
    iter: poem::http::header::ValueIter<'a, HeaderValue>,
}

impl<'a> Iterator for GetAll<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        for value in &mut self.iter {
            if let Ok(value) = value.to_str() {
                return Some(value);
            }
        }
        None
    }
}

/// A view to all binary values stored in a single entry.
pub struct GetBinaryAll<'a> {
    iter: poem::http::header::ValueIter<'a, HeaderValue>,
}

impl<'a> Iterator for GetBinaryAll<'a> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        for value in &mut self.iter {
            if let Ok(value) = STANDARD_NO_PAD.decode(value.as_bytes()) {
                return Some(value);
            }
        }
        None
    }
}
