use crate::{HeaderName, HeaderValue};
use std::iter::FromIterator;

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct HeaderMap(pub(crate) http::header::HeaderMap);

impl HeaderMap {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of headers the map can hold without reallocating.
    ///
    /// This number is an approximation as certain usage patterns could cause additional allocations before the returned capacity is filled.
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Reserves capacity for at least `additional` more headers to be inserted
    /// into the `HeaderMap`.
    ///
    /// The header map may reserve more space to avoid frequent reallocations.
    /// Like with `with_capacity`, this will be a "best effort" to avoid
    /// allocations until `additional` more headers are inserted. Certain usage
    /// patterns could cause additional allocations before the number is
    /// reached.
    ///
    /// # Panics
    ///
    /// Panics if the new allocation size overflows `usize`.
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    /// Returns a reference to the value associated with the key.
    ///
    /// If there are multiple values associated with the key, then the first one
    /// is returned. Use `get_all` to get all values associated with a given
    /// key. Returns `None` if there are no values associated with the key.
    pub fn get(&self, key: HeaderName) -> Option<HeaderValue> {
        self.0.get(key.0).cloned().map(HeaderValue)
    }

    /// Returns a view of all values associated with a key.
    ///
    /// The returned view does not incur any allocations and allows iterating
    /// the values associated with the key.
    pub fn get_all(&self, key: HeaderName) -> impl Iterator<Item = HeaderValue> + '_ {
        self.0.get_all(key.0).into_iter().cloned().map(HeaderValue)
    }

    /// Returns true if the map contains a value for the specified key.
    pub fn contains_key(&self, key: HeaderName) -> bool {
        self.0.contains_key(key.0)
    }

    /// An iterator visiting all key-value pairs.
    ///
    /// The iteration order is arbitrary, but consistent across platforms for
    /// the same crate version. Each key will be yielded once per associated
    /// value. So, if a key has 3 associated values, it will be yielded 3 times.
    pub fn iter(&self) -> impl Iterator<Item = (HeaderName, HeaderValue)> + '_ {
        self.0
            .iter()
            .map(|(key, value)| (HeaderName(key.clone()), HeaderValue(value.clone())))
    }

    /// An iterator visiting all values.
    ///
    /// The iteration order is arbitrary, but consistent across platforms for
    /// the same crate version.
    pub fn keys(&self) -> impl Iterator<Item = HeaderName> + '_ {
        self.0.keys().cloned().map(HeaderName)
    }

    /// An iterator visiting all values mutably.
    ///
    /// The iteration order is arbitrary, but consistent across platforms for
    /// the same crate version.
    pub fn values(&self) -> impl Iterator<Item = HeaderValue> + '_ {
        self.0.values().cloned().map(HeaderValue)
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not previously have this key present, then `None` is
    /// returned.
    ///
    /// If the map did have this key present, the new value is associated with
    /// the key and all previous values are removed. **Note** that only a single
    /// one of the previous values is returned. If there are multiple values
    /// that have been previously associated with the key, then the first one is
    /// returned.
    ///
    /// The key is not updated, though; this matters for types that can be `==`
    /// without being identical.
    pub fn insert(&mut self, key: HeaderName, value: HeaderValue) -> Option<HeaderValue> {
        self.0.insert(key.0, value.0).map(HeaderValue)
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not previously have this key present, then `false` is
    /// returned.
    ///
    /// If the map did have this key present, the new value is pushed to the end
    /// of the list of values currently associated with the key. The key is not
    /// updated, though; this matters for types that can be `==` without being
    /// identical.
    pub fn append(&mut self, key: HeaderName, value: HeaderValue) -> bool {
        self.0.append(key.0, value.0)
    }

    /// Removes a key from the map, returning the value associated with the key.
    ///
    /// Returns `None` if the map does not contain the key. If there are
    /// multiple values associated with the key, then the first one is returned.
    pub fn remove(&mut self, key: HeaderName) -> Option<HeaderValue> {
        self.0.remove(key.0).map(HeaderValue)
    }

    /// Remove the entry from the map.
    ///
    /// All values associated with the entry are removed and the first one is
    /// returned. See [HeaderMap::remove_entry_mult] for an API that returns all values.
    pub fn remove_entry(&mut self, key: HeaderName) -> Option<HeaderValue> {
        if let http::header::Entry::Occupied(e) = self.0.entry(key.0) {
            let (_, value) = e.remove_entry();
            Some(HeaderValue(value))
        } else {
            None
        }
    }

    /// Remove the entry from the map.
    ///
    /// The key and all values associated with the entry are removed and
    /// returned.
    pub fn remove_entry_mult(&mut self, key: HeaderName) -> impl Iterator<Item = HeaderValue> + '_ {
        if let http::header::Entry::Occupied(e) = self.0.entry(key.0) {
            let (_, values) = e.remove_entry_mult();
            Box::new(values.into_iter().map(HeaderValue)) as Box<dyn Iterator<Item = HeaderValue>>
        } else {
            Box::new(std::iter::empty())
        }
    }
}

impl FromIterator<(HeaderName, HeaderValue)> for HeaderMap {
    fn from_iter<T: IntoIterator<Item = (HeaderName, HeaderValue)>>(iter: T) -> Self {
        let mut map = HeaderMap::new();
        for (name, value) in iter {
            map.append(name, value);
        }
        map
    }
}

impl<'a> IntoIterator for &'a HeaderMap {
    type Item = (HeaderName, HeaderValue);
    type IntoIter = Box<dyn Iterator<Item = (HeaderName, HeaderValue)> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(
            self.0
                .iter()
                .map(|(name, value)| (HeaderName(name.clone()), HeaderValue(value.clone()))),
        )
    }
}

impl Extend<(HeaderName, HeaderValue)> for HeaderMap {
    fn extend<T: IntoIterator<Item = (HeaderName, HeaderValue)>>(&mut self, iter: T) {
        for (name, value) in iter {
            self.append(name, value);
        }
    }
}
