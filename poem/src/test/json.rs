use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{Map, Value};

/// A JSON object for testing.
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
pub struct TestJson(Value);

impl TestJson {
    /// Returns a reference the value.
    #[inline]
    pub fn value(&self) -> TestJsonValue<'_> {
        TestJsonValue(&self.0)
    }
}

macro_rules! impl_types {
    ($($(#[$docs:meta])* ($ty:ty, $name:ident, $method:ident)),*) => {
        $(
        $(#[$docs])*
        pub fn $name(&self) -> $ty {
            self.0.$method().expect(stringify!($name))
        }
        )*
    };
}

macro_rules! impl_assert_types {
    ($($(#[$docs:meta])* ($ty:ty, $name:ident, $method:ident)),*) => {
        $(
        $(#[$docs])*
        pub fn $name(&self, value: $ty) {
            assert_eq!(self.$method(), value);
        }
        )*
    };
}

macro_rules! impl_array_types {
    ($($(#[$docs:meta])* ($ty:ty, $name:ident, $method:ident)),*) => {
        $(
        $(#[$docs])*
        pub fn $name(&self) -> Vec<$ty> {
            self.array().iter().map(|value| value.$method()).collect()
        }
        )*
    };
}

macro_rules! impl_assert_array_types {
    ($($(#[$docs:meta])* ($ty:ty, $name:ident, $method:ident)),*) => {
        $(
        $(#[$docs])*
        pub fn $name(&self, values: &[$ty]) {
            assert_eq!(self.$method(), values);
        }
        )*
    };
}

/// A JSON value.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct TestJsonValue<'a>(&'a Value);

impl<'a> PartialEq<Value> for TestJsonValue<'a> {
    fn eq(&self, other: &Value) -> bool {
        self.0 == other
    }
}

impl<'a> TestJsonValue<'a> {
    impl_types!(
        /// Returns the `i64` value.
        (i64, i64, as_i64),
        /// Returns the `f64` value.
        (f64, f64, as_f64),
        /// Returns the `f64` value.
        (bool, bool, as_bool)
    );

    impl_array_types!(
        /// Returns the `i64` array.
        (i64, i64_array, i64),
        /// Returns the `i64` array.
        (f64, f64_array, f64),
        /// Returns the `i64` array.
        (bool, bool_array, bool)
    );

    impl_assert_types!(
        /// Asserts that value is `integer` and it equals to `value`.
        (i64, assert_i64, i64),
        /// Asserts that value is `boolean` and it equals to `value`.
        (bool, assert_bool, bool),
        /// Asserts that value is `string` and it equals to `value`.
        (&str, assert_string, string)
    );

    impl_assert_array_types!(
        /// Asserts that value is `integer` array and it equals to `values`.
        (i64, assert_i64_array, i64_array),
        /// Asserts that value is `boolean` array and it equals to `values`.
        (bool, assert_bool_array, bool_array),
        /// Asserts that value is `string` array and it equals to `values`.
        (&str, assert_string_array, string_array)
    );

    /// Asserts that value is `float` and it equals to `value`.
    pub fn assert_f64(&self, value: f64) {
        assert!((self.f64() - value).abs() < f64::EPSILON);
    }

    /// Asserts that value is `float` array and it equals to `values`.
    pub fn assert_f64_array(&self, values: &[f64]) {
        assert!(self
            .f64_array()
            .iter()
            .zip(values)
            .all(|(a, b)| (*a - *b).abs() < f64::EPSILON));
    }

    /// Returns the `string` value.
    pub fn string(&self) -> &'a str {
        self.0.as_str().expect("string")
    }

    /// Returns the `string` array.
    pub fn string_array(&self) -> Vec<&'a str> {
        self.array().iter().map(|value| value.string()).collect()
    }

    /// Asserts that the value is an array and return `TestJsonArray`.
    pub fn array(&self) -> TestJsonArray<'a> {
        TestJsonArray(self.0.as_array().expect("array"))
    }

    /// Asserts that the value is an object and return `TestJsonArray`.
    pub fn object(&self) -> TestJsonObject<'a> {
        TestJsonObject(self.0.as_object().expect("object"))
    }

    /// Asserts that the value is an object array and return
    /// `Vec<TestJsonObject>`.
    pub fn object_array(&self) -> Vec<TestJsonObject<'a>> {
        self.array().iter().map(|value| value.object()).collect()
    }

    /// Asserts that the value is null.
    pub fn assert_null(&self) {
        assert!(self.0.is_null())
    }

    /// Asserts that the value is not null.
    pub fn assert_not_null(&self) {
        assert!(!self.0.is_null())
    }

    /// Deserialize the value to `T`.
    pub fn deserialize<T: DeserializeOwned>(&self) -> T {
        serde_json::from_value(self.0.clone()).expect("valid json")
    }
}

/// A JSON array.
#[derive(Debug, Copy, Clone)]
pub struct TestJsonArray<'a>(&'a [Value]);

impl<'a, T> PartialEq<T> for TestJsonArray<'a>
where
    T: AsRef<[Value]>,
{
    fn eq(&self, other: &T) -> bool {
        self.0 == other.as_ref()
    }
}

impl<'a> TestJsonArray<'a> {
    /// Returns the number of elements in the array.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if the array contains no elements.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the element at index `idx`.
    pub fn get(&self, idx: usize) -> TestJsonValue<'a> {
        self.get_opt(idx)
            .unwrap_or_else(|| panic!("expect index `{}`", idx))
    }

    /// Returns the element at index `idx`, or `None` if the element does not
    /// exists exists.
    pub fn get_opt(&self, idx: usize) -> Option<TestJsonValue<'a>> {
        self.0.get(idx).map(TestJsonValue)
    }

    /// Returns an iterator over the array.
    pub fn iter(&self) -> impl Iterator<Item = TestJsonValue<'a>> {
        self.0.iter().map(TestJsonValue)
    }

    /// Asserts the array length is equals to `len`.
    pub fn assert_len(&self, len: usize) {
        assert_eq!(self.len(), len);
    }

    /// Asserts the array is empty.
    pub fn assert_is_empty(&self) {
        assert!(self.is_empty());
    }

    /// Asserts the array contains values that satisfies a predicate.
    pub fn assert_contains(&self, f: impl FnMut(TestJsonValue<'_>) -> bool) {
        assert!(self.0.iter().map(TestJsonValue).any(f));
    }

    /// Asserts the array contains exactly one value that satisfies a
    /// predicate.
    pub fn assert_contains_exactly_one(&self, f: impl Fn(TestJsonValue<'_>) -> bool) {
        assert_eq!(
            self.0
                .iter()
                .map(TestJsonValue)
                .filter(|value| f(*value))
                .count(),
            1
        );
    }
}

/// A JSON object.
#[derive(Debug, Copy, Clone)]
pub struct TestJsonObject<'a>(&'a Map<String, Value>);

impl<'a> TestJsonObject<'a> {
    /// Returns the number of elements in the object.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if the object contains no elements.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the element corresponding to the `name`.
    pub fn get(&self, name: impl AsRef<str>) -> TestJsonValue<'a> {
        let name = name.as_ref();
        self.get_opt(name)
            .unwrap_or_else(|| panic!("expect key `{}`", name))
    }

    /// Returns the element corresponding to the `name`, or `None` if the
    /// element does not exists exists.
    pub fn get_opt(&self, name: impl AsRef<str>) -> Option<TestJsonValue<'a>> {
        self.0.get(name.as_ref()).map(TestJsonValue)
    }

    /// Returns an iterator over the object.
    pub fn iter(&self) -> impl Iterator<Item = (&String, TestJsonValue<'a>)> {
        self.0.iter().map(|(k, v)| (k, TestJsonValue(v)))
    }

    /// Asserts the object length is equals to `len`.
    pub fn assert_len(&self, len: usize) {
        assert_eq!(self.len(), len);
    }

    /// Asserts the object is empty.
    pub fn assert_is_empty(&self) {
        assert!(self.is_empty());
    }
}
